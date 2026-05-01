use crate::error::ApiResult;
use crate::models::search::*;
use crate::services::graph_expand::GraphExpandService;
use crate::services::neo4j::Neo4jService;
use crate::services::rerank::RerankService;
use crate::services::vector_search::VectorSearchService;
use regex::Regex;
use std::{collections::HashMap, sync::Arc, time::Instant};

pub struct SearchService {
    neo4j: Arc<Neo4jService>,
    vector: Option<Arc<VectorSearchService>>,
    graph_expand: Arc<GraphExpandService>,
    rerank: Option<Arc<RerankService>>,
}

#[derive(Debug, Clone)]
struct QueryPlan {
    analysis: SearchAnalysis,
    intent: SearchIntent,
    retrieval_query: String,
    chapter_filter: Option<String>,
    authority_filter: Option<String>,
}

impl SearchService {
    pub fn new(
        neo4j: Arc<Neo4jService>,
        vector: Option<Arc<VectorSearchService>>,
        graph_expand: Arc<GraphExpandService>,
        rerank: Option<Arc<RerankService>>,
    ) -> Self {
        Self {
            neo4j,
            vector,
            graph_expand,
            rerank,
        }
    }

    fn resolve_mode(requested_mode: SearchMode, plan: &QueryPlan) -> SearchMode {
        match requested_mode {
            SearchMode::Auto
                if matches!(plan.intent, SearchIntent::Citation | SearchIntent::Chapter)
                    && plan.analysis.residual_text.is_none() =>
            {
                SearchMode::Citation
            }
            SearchMode::Auto => SearchMode::Hybrid,
            mode => mode,
        }
    }

    pub async fn search(&self, query: SearchQuery) -> ApiResult<SearchResponse> {
        let started_at = Instant::now();
        let raw_query = query.q.clone();
        let mut plan =
            analyze_search_query_with_authority(&raw_query, query.authority_family.as_deref());
        let requested_mode = query.mode.unwrap_or_default();
        let mode = Self::resolve_mode(requested_mode, &plan);
        let mut filters = SearchRetrievalFilters::from_query(&query);
        if filters.chapter.is_none() {
            filters.chapter = plan.chapter_filter.clone();
        }
        if filters.authority_family.is_none() {
            filters.authority_family = plan.authority_filter.clone();
        }
        plan.analysis.applied_filters = filters.applied_filter_names();

        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        let offset = query.offset.unwrap_or(0);
        let candidate_limit = self
            .rerank
            .as_ref()
            .map(|r| r.candidates_limit())
            .unwrap_or(limit as usize * 4)
            .max(limit as usize)
            .min(250);
        let pre_expand_limit = (candidate_limit * 2).min(250).max(limit as usize);

        let mut warnings = Vec::new();
        let mut retrieval = RetrievalInfo::default();
        let mut results = Vec::new();

        if plan.analysis.normalized_query.is_empty() {
            plan.analysis.timings.total_ms = started_at.elapsed().as_millis() as u64;
            return Ok(SearchResponse {
                query: raw_query,
                mode,
                total: 0,
                limit,
                offset,
                results,
                facets: Some(self.neo4j.aggregate_facets(&[])),
                warnings,
                analysis: plan.analysis,
                retrieval,
                embeddings: Some(self.embeddings_info()),
                rerank: Some(self.default_rerank_info()),
            });
        }

        if !plan.retrieval_query.is_empty() && mode != SearchMode::Citation {
            match self
                .neo4j
                .expand_query_terms(&plan.retrieval_query, &filters, 8)
                .await
            {
                Ok(expansion_terms) => {
                    plan.analysis.expansion_count = expansion_terms.len();
                    plan.analysis.expansion_terms = expansion_terms;
                }
                Err(e) => warnings.push(format!("Graph term expansion failed: {}", e)),
            }
        }

        let expanded_query =
            build_expanded_retrieval_query(&plan.retrieval_query, &plan.analysis.expansion_terms);
        let has_exact_signal = !plan.analysis.citations.is_empty()
            || !plan.analysis.ranges.is_empty()
            || (matches!(plan.intent, SearchIntent::Chapter)
                && plan.analysis.residual_text.is_none());
        let should_run_exact =
            matches!(mode, SearchMode::Citation | SearchMode::Hybrid) && has_exact_signal;
        let should_run_keyword = matches!(mode, SearchMode::Keyword | SearchMode::Hybrid)
            && !plan.retrieval_query.is_empty();
        let should_run_vector = matches!(mode, SearchMode::Semantic | SearchMode::Hybrid)
            && !plan.retrieval_query.is_empty();

        let retrieval_started = Instant::now();

        if should_run_exact {
            for range in &plan.analysis.ranges {
                match self
                    .neo4j
                    .search_citation_range(range, candidate_limit as u32)
                    .await
                {
                    Ok(range_results) => {
                        retrieval.exact_candidates += range_results.len();
                        for mut res in range_results {
                            mark_exact_result(&mut res, 85.0, "exact");
                            results.push(res);
                        }
                    }
                    Err(e) => warnings.push(format!("Citation range lookup failed: {}", e)),
                }
            }

            for exact_query in exact_queries_for_plan(&plan) {
                let exact_authority =
                    authority_family_for_exact(&exact_query, &plan.analysis.citations)
                        .or(plan.authority_filter.as_deref());
                match self.neo4j.search_exact(&exact_query, exact_authority).await {
                    Ok(mut exact_results) => {
                        if exact_results.is_empty() {
                            if let Some(parent_query) =
                                parent_query_for_exact(&exact_query, &plan.analysis.citations)
                            {
                                match self
                                    .neo4j
                                    .search_exact(&parent_query, exact_authority)
                                    .await
                                {
                                    Ok(parent_results) => {
                                        exact_results = parent_results;
                                        for res in &mut exact_results {
                                            mark_exact_result(res, 80.0, "parent");
                                        }
                                    }
                                    Err(e) => warnings
                                        .push(format!("Parent citation lookup failed: {}", e)),
                                }
                            }
                        }

                        retrieval.exact_candidates += exact_results.len();
                        for mut res in exact_results {
                            if res.rank_source.as_deref() != Some("parent") {
                                mark_exact_result(&mut res, 100.0, "exact");
                            }
                            results.push(res);
                        }
                    }
                    Err(e) => warnings.push(format!("Exact citation lookup failed: {}", e)),
                }
            }
        }

        if should_run_keyword {
            match self
                .neo4j
                .search_fulltext(&plan.retrieval_query, &filters, candidate_limit as u32)
                .await
            {
                Ok(mut keyword_results) => {
                    retrieval.fulltext_candidates += keyword_results.len();
                    for res in &mut keyword_results {
                        if res.rank_source.is_none() {
                            res.rank_source = Some("keyword".to_string());
                        }
                    }
                    results.extend(keyword_results);
                }
                Err(e) => warnings.push(format!("Full-text search failed: {}", e)),
            }

            if expanded_query != plan.retrieval_query {
                match self
                    .neo4j
                    .search_fulltext(
                        &expanded_query,
                        &filters,
                        (candidate_limit / 2).max(5) as u32,
                    )
                    .await
                {
                    Ok(mut expanded_results) => {
                        retrieval.fulltext_candidates += expanded_results.len();
                        for res in &mut expanded_results {
                            res.score *= 0.72;
                            res.fulltext_score = res.fulltext_score.map(|score| score * 0.72);
                            res.rank_source = Some("graph-expanded".to_string());
                            if let Some(breakdown) = &mut res.score_breakdown {
                                breakdown.keyword = res.fulltext_score;
                                breakdown.expansion = Some(0.25);
                            }
                        }
                        results.extend(expanded_results);
                    }
                    Err(e) => warnings.push(format!("Expanded full-text search failed: {}", e)),
                }
            }
        }

        if should_run_vector {
            if let Some(vector) = &self.vector {
                match vector
                    .search_chunks(&plan.retrieval_query, candidate_limit, &filters)
                    .await
                {
                    Ok(vector_results) => {
                        retrieval.vector_candidates = vector_results.len();
                        results.extend(vector_results);
                    }
                    Err(e) => {
                        warnings.push(format!(
                            "Vector search unavailable; using keyword + graph retrieval. {}",
                            e
                        ));
                        if mode == SearchMode::Semantic {
                            match self
                                .neo4j
                                .search_fulltext(
                                    &plan.retrieval_query,
                                    &filters,
                                    candidate_limit as u32,
                                )
                                .await
                            {
                                Ok(mut keyword_results) => {
                                    retrieval.fulltext_candidates += keyword_results.len();
                                    for res in &mut keyword_results {
                                        res.rank_source = Some("keyword-fallback".to_string());
                                    }
                                    results.extend(keyword_results);
                                }
                                Err(e) => warnings.push(format!("Keyword fallback failed: {}", e)),
                            }
                        }
                    }
                }
            } else {
                warnings.push(
                    "Vector search unavailable; using keyword + graph retrieval.".to_string(),
                );
                if mode == SearchMode::Semantic {
                    match self
                        .neo4j
                        .search_fulltext(&plan.retrieval_query, &filters, candidate_limit as u32)
                        .await
                    {
                        Ok(mut keyword_results) => {
                            retrieval.fulltext_candidates += keyword_results.len();
                            for res in &mut keyword_results {
                                res.rank_source = Some("keyword-fallback".to_string());
                            }
                            results.extend(keyword_results);
                        }
                        Err(e) => warnings.push(format!("Keyword fallback failed: {}", e)),
                    }
                }
            }
        }
        plan.analysis.timings.retrieval_ms = retrieval_started.elapsed().as_millis() as u64;

        let mut candidates = self.rank_and_dedupe(results, plan.intent);
        self.apply_filters(&mut candidates, &filters, FilterStage::Early);
        retrieval.filtered_candidates = candidates.len();
        if candidates.len() > pre_expand_limit {
            candidates.truncate(pre_expand_limit);
        }
        retrieval.capped_candidates = candidates.len();

        let graph_started = Instant::now();
        retrieval.graph_expanded_candidates =
            match self.graph_expand.expand_candidates(&mut candidates).await {
                Ok(count) => count,
                Err(e) => {
                    warnings.push(format!("Graph expansion failed: {}", e));
                    0
                }
            };
        plan.analysis.timings.graph_ms = graph_started.elapsed().as_millis() as u64;
        self.apply_filters(&mut candidates, &filters, FilterStage::Late);
        self.apply_expansion_boosts(&mut candidates, &plan.analysis.expansion_terms);

        for candidate in &mut candidates {
            let boosts = self.calculate_legal_boosts(candidate, plan.intent);
            candidate.score += boosts;
            match &mut candidate.score_breakdown {
                Some(breakdown) => breakdown.authority = Some(boosts),
                None => {
                    candidate.score_breakdown = Some(ScoreBreakdown {
                        exact: None,
                        keyword: candidate.fulltext_score,
                        vector: candidate.vector_score,
                        rerank: None,
                        graph: candidate.graph_score,
                        authority: Some(boosts),
                        expansion: None,
                        penalties: None,
                    });
                }
            }
        }
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let facets = Some(self.neo4j.aggregate_facets(&candidates));
        let mut rerank_info = None;
        let mut rerank_applied = false;

        if let Some(reranker) = &self.rerank {
            let candidate_count = candidates.len().min(reranker.candidates_limit());
            if candidate_count > 0 && mode != SearchMode::Citation {
                let rerank_slice = candidate_count;
                let rerank_started = Instant::now();
                match reranker
                    .rerank(&plan.retrieval_query, &candidates[..rerank_slice])
                    .await
                {
                    Ok(output) => {
                        let mut reranked = output
                            .results
                            .iter()
                            .map(|result| (result.index, result.score))
                            .collect::<Vec<_>>();
                        reranked.sort_by(|a, b| {
                            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        self.apply_authoritative_rerank(
                            &mut candidates,
                            rerank_slice,
                            &reranked,
                            plan.intent,
                        );
                        rerank_applied = !reranked.is_empty();
                        retrieval.reranked_candidates = output.results.len();
                        plan.analysis.timings.rerank_ms =
                            rerank_started.elapsed().as_millis() as u64;

                        rerank_info = Some(RerankInfo {
                            enabled: true,
                            model: Some(reranker.model().to_string()),
                            candidate_count: Some(candidate_count),
                            returned_count: Some(output.results.len()),
                            total_tokens: Some(output.total_tokens),
                        });
                    }
                    Err(e) => {
                        warnings.push(format!(
                            "Rerank failed; using internal legal ranking. {}",
                            e
                        ));
                        plan.analysis.timings.rerank_ms =
                            rerank_started.elapsed().as_millis() as u64;
                        rerank_info = Some(RerankInfo {
                            enabled: true,
                            model: Some(reranker.model().to_string()),
                            candidate_count: Some(candidate_count),
                            returned_count: None,
                            total_tokens: None,
                        });
                    }
                }
            }
        }

        self.enforce_exact_priority(&mut candidates);
        if !rerank_applied {
            candidates.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        if rerank_info.is_none() {
            rerank_info = Some(self.default_rerank_info());
        }

        let total = candidates.len();
        let final_results: Vec<SearchResult> = candidates
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        plan.analysis.timings.total_ms = started_at.elapsed().as_millis() as u64;

        Ok(SearchResponse {
            query: raw_query,
            mode,
            total,
            limit,
            offset,
            results: final_results,
            facets,
            warnings,
            analysis: plan.analysis,
            retrieval,
            embeddings: Some(self.embeddings_info()),
            rerank: rerank_info,
        })
    }

    fn embeddings_info(&self) -> EmbeddingsInfo {
        EmbeddingsInfo {
            enabled: self.vector.is_some(),
            model: self.vector.as_ref().map(|v| v.model().to_string()),
            profile: self.vector.as_ref().map(|v| v.profile().to_string()),
            dimension: self.vector.as_ref().map(|v| v.dimension()),
        }
    }

    fn default_rerank_info(&self) -> RerankInfo {
        RerankInfo {
            enabled: self.rerank.is_some(),
            model: self.rerank.as_ref().map(|r| r.model().to_string()),
            candidate_count: None,
            returned_count: None,
            total_tokens: None,
        }
    }

    fn apply_expansion_boosts(
        &self,
        candidates: &mut [SearchResult],
        terms: &[QueryExpansionTerm],
    ) {
        if terms.is_empty() {
            return;
        }

        let needles = terms
            .iter()
            .flat_map(|term| {
                [
                    term.term.to_ascii_lowercase(),
                    term.normalized_term.clone().unwrap_or_default(),
                ]
            })
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();

        for candidate in candidates {
            if self.is_exact_candidate(candidate) {
                continue;
            }

            let mut haystack = candidate.snippet.to_ascii_lowercase();
            if let Some(title) = &candidate.title {
                haystack.push(' ');
                haystack.push_str(&title.to_ascii_lowercase());
            }
            if let Some(citation) = &candidate.citation {
                haystack.push(' ');
                haystack.push_str(&citation.to_ascii_lowercase());
            }

            let matches = needles
                .iter()
                .filter(|needle| haystack.contains(needle.as_str()))
                .count();
            if matches == 0 {
                continue;
            }

            let boost = (matches as f32 * 0.2).min(0.8);
            candidate.score += boost;
            match &mut candidate.score_breakdown {
                Some(breakdown) => {
                    breakdown.expansion = Some(breakdown.expansion.unwrap_or(0.0) + boost)
                }
                None => {
                    candidate.score_breakdown = Some(ScoreBreakdown {
                        exact: None,
                        keyword: candidate.fulltext_score,
                        vector: candidate.vector_score,
                        rerank: candidate.rerank_score,
                        graph: candidate.graph_score,
                        authority: None,
                        expansion: Some(boost),
                        penalties: None,
                    });
                }
            }
        }
    }

    fn calculate_legal_boosts(&self, res: &SearchResult, intent: SearchIntent) -> f32 {
        let mut boost = 0.0;

        if self.is_exact_candidate(res) {
            boost += 50.0;
        }

        if res.source_backed {
            boost += 1.0;
        }

        if res.status.as_deref() == Some("active") {
            boost += 0.5;
        }

        if res.kind == "provision" {
            boost += 0.35;
        } else if res.kind == "chunk" {
            boost -= 0.2;
        }

        if let Some(graph) = &res.graph {
            if let Some(count) = graph.citation_count {
                boost += (count as f32).log10().max(0.0) * 0.2;
            }
            if let Some(count) = graph.connected_node_count {
                boost += (count as f32).log10().max(0.0) * 0.25;
            }
        }

        boost += self.intent_boost(res, intent);

        if !res.qc_warnings.is_empty() {
            boost -= 0.5 * res.qc_warnings.len() as f32;
        }

        let status = res.status.as_deref().unwrap_or_default().to_lowercase();
        if matches!(status.as_str(), "repealed" | "renumbered" | "stale")
            && intent != SearchIntent::History
        {
            boost -= 2.0;
        }

        boost
    }

    fn intent_boost(&self, res: &SearchResult, intent: SearchIntent) -> f32 {
        let mut haystack = res.kind.to_lowercase();
        haystack.push(' ');
        haystack.push_str(
            &res.semantic_types
                .iter()
                .map(|s| s.to_lowercase())
                .collect::<Vec<_>>()
                .join(" "),
        );
        haystack.push(' ');
        haystack.push_str(&res.snippet.to_lowercase());

        match intent {
            SearchIntent::Definition if haystack.contains("definition") => 1.0,
            SearchIntent::Deadline
                if haystack.contains("deadline")
                    || haystack.contains("temporal")
                    || haystack.contains("notice") =>
            {
                1.0
            }
            SearchIntent::Penalty
                if haystack.contains("penalty")
                    || haystack.contains("fine")
                    || haystack.contains("civil penalty") =>
            {
                1.0
            }
            SearchIntent::Notice if haystack.contains("notice") => 0.75,
            SearchIntent::Actor
                if haystack.contains("obligation") || haystack.contains("shall") =>
            {
                0.75
            }
            SearchIntent::History
                if haystack.contains("operative")
                    || haystack.contains("effective")
                    || haystack.contains("amend")
                    || haystack.contains("repeal") =>
            {
                1.0
            }
            _ => 0.0,
        }
    }

    fn enforce_exact_priority(&self, candidates: &mut [SearchResult]) {
        for candidate in candidates {
            if self.is_exact_candidate(candidate) {
                candidate.score = candidate.score.max(100.0);
            }
        }
    }

    fn apply_filters(
        &self,
        candidates: &mut Vec<SearchResult>,
        filters: &SearchRetrievalFilters,
        stage: FilterStage,
    ) {
        candidates.retain(|candidate| {
            if stage.includes_early() {
                if let Some(result_type) = filters.result_type.as_deref() {
                    if !self.matches_result_type(candidate, result_type) {
                        return false;
                    }
                }
                if let Some(authority_family) = filters.authority_family.as_deref() {
                    if !candidate_matches_authority_family(candidate, authority_family) {
                        return false;
                    }
                }
                if let Some(chapter) = filters.chapter.as_deref() {
                    if candidate.chapter.as_deref() != Some(chapter) {
                        return false;
                    }
                }
                if let Some(status) = filters.status.as_deref() {
                    if candidate.status.as_deref() != Some(status) {
                        return false;
                    }
                }
                if filters.current_only
                    && candidate
                        .status
                        .as_deref()
                        .map(|value| value != "active")
                        .unwrap_or(false)
                {
                    return false;
                }
                if filters.source_backed_only && !candidate.source_backed {
                    return false;
                }
            }

            if !stage.includes_late() {
                return true;
            }

            if let Some(result_type) = filters.result_type.as_deref() {
                if !self.matches_result_type(candidate, result_type) {
                    return false;
                }
            }
            if let Some(authority_family) = filters.authority_family.as_deref() {
                if !candidate_matches_authority_family(candidate, authority_family) {
                    return false;
                }
            }
            if let Some(chapter) = filters.chapter.as_deref() {
                if candidate.chapter.as_deref() != Some(chapter) {
                    return false;
                }
            }
            if let Some(status) = filters.status.as_deref() {
                if candidate.status.as_deref() != Some(status) {
                    return false;
                }
            }
            if filters.current_only {
                if candidate
                    .status
                    .as_deref()
                    .map(|value| value != "active")
                    .unwrap_or(false)
                {
                    return false;
                }
            }
            if filters.source_backed_only && !candidate.source_backed {
                return false;
            }
            if filters.has_citations && !self.has_citations(candidate) {
                return false;
            }
            if filters.has_deadlines
                && !self.has_semantic_signal(candidate, &["deadline", "temporaleffect"])
            {
                return false;
            }
            if filters.has_penalties && !self.has_semantic_signal(candidate, &["penalty"]) {
                return false;
            }
            if filters.needs_review && candidate.qc_warnings.is_empty() {
                return false;
            }
            if let Some(semantic_type) = filters.semantic_type.as_deref() {
                if !self.has_semantic_signal(candidate, &[semantic_type]) {
                    return false;
                }
            }
            true
        });
    }

    fn apply_authoritative_rerank(
        &self,
        candidates: &mut Vec<SearchResult>,
        rerank_slice: usize,
        reranked: &[(usize, f32)],
        intent: SearchIntent,
    ) {
        let positions: HashMap<usize, (usize, f32)> = reranked
            .iter()
            .enumerate()
            .map(|(position, (index, score))| (*index, (position, *score)))
            .collect();

        for (idx, candidate) in candidates.iter_mut().take(rerank_slice).enumerate() {
            candidate.pre_rerank_score = Some(candidate.score);
            if let Some((_, rerank_score)) = positions.get(&idx) {
                let authority = self.calculate_legal_boosts(candidate, intent);
                candidate.rerank_score = Some(*rerank_score);
                candidate.score = 10.0 + (*rerank_score * 10.0) + authority;
                if candidate.rank_source.as_deref() != Some("exact") {
                    candidate.rank_source = Some("rerank".to_string());
                }
                match &mut candidate.score_breakdown {
                    Some(breakdown) => {
                        breakdown.rerank = Some(*rerank_score);
                        breakdown.authority = Some(authority);
                    }
                    None => {
                        candidate.score_breakdown = Some(ScoreBreakdown {
                            exact: None,
                            keyword: candidate.fulltext_score,
                            vector: candidate.vector_score,
                            rerank: Some(*rerank_score),
                            graph: candidate.graph_score,
                            authority: Some(authority),
                            expansion: None,
                            penalties: None,
                        });
                    }
                }
            }
        }

        let ordered = order_candidates_after_rerank(std::mem::take(candidates), &positions);
        candidates.extend(ordered);
    }

    fn matches_result_type(&self, candidate: &SearchResult, result_type: &str) -> bool {
        let kind = candidate.kind.to_lowercase();
        let expected = result_type.to_lowercase();
        kind == expected
            || match expected.as_str() {
                "statute" => kind == "legaltextidentity" || kind == "legaltextversion",
                "court_rule" | "courtrule" | "rule" => matches!(
                    kind.as_str(),
                    "court_rule"
                        | "courtrule"
                        | "utcrrule"
                        | "utcrruleversion"
                        | "court_rule_provision"
                        | "utcrprovision"
                ),
                "provision" => matches!(kind.as_str(), "court_rule_provision" | "utcrprovision"),
                "semantic" => matches!(
                    kind.as_str(),
                    "legalsemanticnode"
                        | "obligation"
                        | "exception"
                        | "deadline"
                        | "penalty"
                        | "remedy"
                        | "requirednotice"
                        | "proceduralrequirement"
                        | "formattingrequirement"
                        | "filingrequirement"
                        | "servicerequirement"
                        | "efilingrequirement"
                        | "certificateofservicerequirement"
                        | "exhibitrequirement"
                        | "protectedinformationrequirement"
                ),
                "definition" | "definedterm" => {
                    kind == "definedterm" || self.has_semantic_signal(candidate, &["definition"])
                }
                "obligation" | "exception" | "deadline" | "penalty" | "remedy" => {
                    self.has_semantic_signal(candidate, &[expected.as_str()])
                }
                "notice" | "requirednotice" => {
                    matches!(kind.as_str(), "requirednotice" | "required_notice")
                        || self.has_semantic_signal(candidate, &["notice", "requirednotice"])
                }
                "history" => matches!(
                    kind.as_str(),
                    "sourcenote"
                        | "source_note"
                        | "statusevent"
                        | "status_event"
                        | "temporaleffect"
                        | "temporal_effect"
                        | "sessionlaw"
                        | "session_law"
                        | "amendment"
                ),
                "source_note" | "sourcenote" => {
                    matches!(kind.as_str(), "sourcenote" | "source_note")
                }
                "temporal_effect" | "temporaleffect" => {
                    matches!(kind.as_str(), "temporaleffect" | "temporal_effect")
                }
                "session_law" | "sessionlaw" => {
                    matches!(kind.as_str(), "sessionlaw" | "session_law")
                }
                "taxrule" | "tax_rule" => matches!(kind.as_str(), "taxrule" | "tax_rule"),
                "moneyamount" | "money_amount" => {
                    matches!(kind.as_str(), "moneyamount" | "money_amount")
                }
                "ratelimit" | "rate_limit" => matches!(kind.as_str(), "ratelimit" | "rate_limit"),
                "legalactor" | "legal_actor" | "actor" => {
                    matches!(kind.as_str(), "legalactor" | "legal_actor")
                }
                "legalaction" | "legal_action" | "action" => {
                    matches!(kind.as_str(), "legalaction" | "legal_action")
                }
                _ => false,
            }
    }

    fn has_citations(&self, candidate: &SearchResult) -> bool {
        candidate
            .graph
            .as_ref()
            .map(|graph| {
                graph.citation_count.unwrap_or(0) > 0 || graph.cited_by_count.unwrap_or(0) > 0
            })
            .unwrap_or(false)
    }

    fn has_semantic_signal(&self, candidate: &SearchResult, needles: &[&str]) -> bool {
        let kind = candidate.kind.to_lowercase();
        let snippet = candidate.snippet.to_lowercase();
        let semantic_types = candidate
            .semantic_types
            .iter()
            .map(|value| value.to_lowercase())
            .collect::<Vec<_>>();

        needles.iter().any(|needle| {
            let needle = needle.to_lowercase();
            kind.contains(&needle)
                || snippet.contains(&needle)
                || semantic_types.iter().any(|value| value.contains(&needle))
        })
    }

    fn is_exact_candidate(&self, candidate: &SearchResult) -> bool {
        is_exact_candidate(candidate)
    }

    pub async fn direct_open(
        &self,
        q: &str,
        authority_family: Option<&str>,
    ) -> ApiResult<DirectOpenResponse> {
        let plan = analyze_search_query_with_authority(q, authority_family);
        let normalized = plan.analysis.normalized_query.clone();
        let authority_family = plan
            .analysis
            .citations
            .first()
            .map(|citation| citation.authority_family.as_str())
            .or(plan.authority_filter.as_deref());
        let citation = plan
            .analysis
            .citations
            .first()
            .map(|citation| citation.normalized.as_str())
            .unwrap_or(normalized.as_str());

        if let Some(result) = self
            .neo4j
            .search_exact_provision(citation, authority_family)
            .await?
        {
            return Ok(direct_response_from_result(
                true,
                DirectMatchType::ExactProvision,
                normalized,
                result,
                None,
            ));
        }

        if let Some(result) = self
            .neo4j
            .search_exact_statute(citation, authority_family)
            .await?
        {
            return Ok(direct_response_from_result(
                true,
                DirectMatchType::ExactStatute,
                normalized,
                result,
                None,
            ));
        }

        if let Some(parent_citation) = plan
            .analysis
            .citations
            .first()
            .and_then(|citation| citation.parent.as_deref())
        {
            if let Some(result) = self
                .neo4j
                .search_exact_statute(parent_citation, authority_family)
                .await?
            {
                let parent = DirectOpenParent {
                    citation: result
                        .citation
                        .clone()
                        .unwrap_or_else(|| parent_citation.to_string()),
                    canonical_id: result.id.clone(),
                    href: result.href.clone(),
                };
                return Ok(direct_response_from_result(
                    true,
                    DirectMatchType::ParentStatute,
                    normalized,
                    result,
                    Some(parent),
                ));
            }
        }

        Ok(DirectOpenResponse {
            matched: false,
            match_type: DirectMatchType::None,
            normalized_query: normalized,
            citation: q.to_string(),
            canonical_id: String::new(),
            href: String::new(),
            parent: None,
        })
    }

    pub async fn suggest(&self, q: &str, limit: Option<u32>) -> ApiResult<Vec<SuggestResult>> {
        let limit = limit.unwrap_or(10);
        self.neo4j.suggest(q, limit).await
    }

    fn rank_and_dedupe(
        &self,
        results: Vec<SearchResult>,
        intent: SearchIntent,
    ) -> Vec<SearchResult> {
        let mut unique: std::collections::HashMap<String, SearchResult> =
            std::collections::HashMap::new();

        for mut res in results {
            res.score += self.intent_boost(&res, intent);
            let key = self.dedupe_key(&res);

            if let Some(existing) = unique.get_mut(&key) {
                let res_priority = self.kind_priority(&res.kind);
                let existing_priority = self.kind_priority(&existing.kind);

                if res_priority > existing_priority
                    || (res_priority == existing_priority && res.score > existing.score)
                {
                    let old = existing.clone();
                    *existing = res;
                    self.merge_scores(existing, &old);
                } else {
                    self.merge_scores(existing, &res);
                }
            } else {
                unique.insert(key, res);
            }
        }

        let mut sorted: Vec<_> = unique.into_values().collect();
        sorted.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
    }

    fn dedupe_key(&self, res: &SearchResult) -> String {
        if let Some(source) = &res.source {
            if let Some(provision_id) = &source.provision_id {
                return format!("provision:{}", provision_id);
            }
            if let Some(version_id) = &source.version_id {
                return format!("version:{}", version_id);
            }
        }

        if let Some(graph) = &res.graph {
            if let Some(provision_id) = &graph.provision_id {
                return format!("provision:{}", provision_id);
            }
            if let Some(version_id) = &graph.version_id {
                return format!("version:{}", version_id);
            }
            if let Some(canonical_id) = &graph.canonical_id {
                return format!("identity:{}", canonical_id);
            }
        }

        if let Some(citation) = &res.citation {
            if res.kind == "statute" || res.kind == "provision" {
                return format!("citation:{}", citation);
            }
        }

        format!("id:{}", res.id)
    }

    fn merge_scores(&self, target: &mut SearchResult, other: &SearchResult) {
        let stronger = target.score.max(other.score);
        let supporting = target.score.min(other.score).max(0.0);
        target.score = stronger + supporting * 0.35 + 0.05;
        target.vector_score = target.vector_score.or(other.vector_score);
        target.fulltext_score = target.fulltext_score.or(other.fulltext_score);
        target.graph_score = target.graph_score.or(other.graph_score);
        target.rerank_score = target.rerank_score.or(other.rerank_score);

        for semantic_type in &other.semantic_types {
            if !target.semantic_types.contains(semantic_type) {
                target.semantic_types.push(semantic_type.clone());
            }
        }

        if target.source.is_none() {
            target.source = other.source.clone();
        }
        if target.graph.is_none() {
            target.graph = other.graph.clone();
        }
        if target.rank_source.as_deref() != Some("exact")
            && other.rank_source.as_deref() == Some("exact")
        {
            target.rank_source = Some("exact".to_string());
        }
    }

    fn kind_priority(&self, kind: &str) -> i32 {
        match kind.to_lowercase().as_str() {
            "provision" | "court_rule_provision" | "utcrprovision" => 10,
            "statute" | "court_rule" | "legaltextidentity" | "legaltextversion" => 9,
            "definition" | "definedterm" => 8,
            "obligation"
            | "exception"
            | "deadline"
            | "penalty"
            | "remedy"
            | "requirednotice"
            | "proceduralrequirement"
            | "formattingrequirement"
            | "filingrequirement"
            | "servicerequirement"
            | "efilingrequirement"
            | "certificateofservicerequirement"
            | "exhibitrequirement"
            | "protectedinformationrequirement"
            | "taxrule"
            | "moneyamount"
            | "ratelimit"
            | "legalactor"
            | "legalaction" => 7,
            "sourcenote" | "sessionlaw" | "amendment" | "temporaleffect" => 6,
            "chunk" | "retrievalchunk" => 5,
            _ => 0,
        }
    }
}

fn direct_response_from_result(
    matched: bool,
    match_type: DirectMatchType,
    normalized_query: String,
    result: SearchResult,
    parent: Option<DirectOpenParent>,
) -> DirectOpenResponse {
    DirectOpenResponse {
        matched,
        match_type,
        normalized_query,
        citation: result.citation.unwrap_or_default(),
        canonical_id: result.id,
        href: result.href,
        parent,
    }
}

fn mark_exact_result(result: &mut SearchResult, score: f32, rank_source: &str) {
    result.rank_source = Some(rank_source.to_string());
    result.score = score + result.score;
    result.score_breakdown = Some(ScoreBreakdown {
        exact: Some(score),
        keyword: None,
        vector: None,
        rerank: None,
        graph: None,
        authority: None,
        expansion: None,
        penalties: None,
    });
}

fn analyze_search_query(q: &str) -> QueryPlan {
    analyze_search_query_with_authority(q, None)
}

fn analyze_search_query_with_authority(q: &str, authority_family: Option<&str>) -> QueryPlan {
    let requested_authority = normalized_authority_filter(authority_family);
    let normalized = normalize_search_query_with_authority(q, requested_authority.as_deref());
    let ranges = parse_citation_ranges(&normalized);
    let citations = parse_query_citations(&normalized);
    let (chapter_prefix, chapter_authority, chapter_residual) = parse_chapter_query(&normalized);

    let residual_text = if let Some(residual) = chapter_residual {
        Some(residual)
    } else if !ranges.is_empty() {
        clean_residual_text(remove_ranges_from_query(&normalized, &ranges))
    } else if !citations.is_empty() {
        clean_residual_text(remove_citations_from_query(&normalized, &citations))
    } else {
        None
    };

    let inferred_chapter = chapter_prefix
        .clone()
        .or_else(|| ranges.first().map(|range| range.chapter.clone()))
        .or_else(|| citations.first().map(|citation| citation.chapter.clone()));

    let inferred_authority_family = chapter_authority
        .clone()
        .or_else(|| ranges.first().map(|range| range.authority_family.clone()))
        .or_else(|| {
            citations
                .first()
                .map(|citation| citation.authority_family.clone())
        })
        .or(requested_authority);

    let chapter_filter = chapter_prefix.clone().or_else(|| {
        residual_text
            .as_ref()
            .and_then(|_| inferred_chapter.clone())
    });

    let intent = if !ranges.is_empty() && residual_text.is_none() {
        SearchIntent::Citation
    } else if !citations.is_empty() && residual_text.is_none() {
        SearchIntent::Citation
    } else if chapter_prefix.is_some() && residual_text.is_none() {
        SearchIntent::Chapter
    } else {
        detect_search_intent(residual_text.as_deref().unwrap_or(&normalized))
    };

    let retrieval_query = residual_text.clone().unwrap_or_else(|| normalized.clone());

    QueryPlan {
        analysis: SearchAnalysis {
            normalized_query: normalized,
            intent: intent.as_str().to_string(),
            inferred_authority_family: inferred_authority_family.clone(),
            citations,
            ranges,
            inferred_chapter,
            residual_text,
            expansion_terms: Vec::new(),
            expansion_count: 0,
            applied_filters: Vec::new(),
            timings: SearchTimingInfo::default(),
        },
        intent,
        retrieval_query,
        chapter_filter,
        authority_filter: inferred_authority_family,
    }
}

fn exact_queries_for_plan(plan: &QueryPlan) -> Vec<String> {
    let mut queries = Vec::new();
    if plan.analysis.ranges.is_empty() {
        for citation in &plan.analysis.citations {
            push_unique(&mut queries, citation.normalized.clone());
        }
    }

    if queries.is_empty()
        && matches!(plan.intent, SearchIntent::Chapter)
        && plan.analysis.residual_text.is_none()
    {
        push_unique(&mut queries, plan.analysis.normalized_query.clone());
    }

    queries
}

fn parent_query_for_exact(exact_query: &str, citations: &[QueryCitation]) -> Option<String> {
    citations
        .iter()
        .find(|citation| citation.normalized == exact_query)
        .and_then(|citation| citation.parent.clone())
}

fn authority_family_for_exact<'a>(
    exact_query: &str,
    citations: &'a [QueryCitation],
) -> Option<&'a str> {
    citations
        .iter()
        .find(|citation| citation.normalized == exact_query)
        .map(|citation| citation.authority_family.as_str())
}

fn build_expanded_retrieval_query(base: &str, terms: &[QueryExpansionTerm]) -> String {
    let mut parts = vec![base.trim().to_string()];
    for term in terms.iter().take(5) {
        if !parts
            .iter()
            .any(|part| part.eq_ignore_ascii_case(&term.term))
        {
            parts.push(term.term.clone());
        }
    }
    parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn normalize_search_query(q: &str) -> String {
    normalize_search_query_with_authority(q, None)
}

fn normalize_search_query_with_authority(q: &str, authority_family: Option<&str>) -> String {
    let mut normalized = q.trim().to_string();

    normalized = normalized
        .replace('“', "\"")
        .replace('”', "\"")
        .replace('‘', "'")
        .replace('’', "'");

    let default_authority = authority_family.unwrap_or("ORS").to_ascii_uppercase();
    let range_re = Regex::new(
        r"(?i)^\s*(?:(ORS|UTCR)\s+)?(\d{1,3}[A-Z]?\.\d{3}(?:\([^)]+\))?)\s+(?:to|through|thru|-|–)\s+(?:(ORS|UTCR)\s+)?(\d{1,3}[A-Z]?\.\d{3}(?:\([^)]+\))?)\s*$",
    )
    .unwrap();
    if let Some(caps) = range_re.captures(&normalized) {
        let authority = caps
            .get(1)
            .or_else(|| caps.get(3))
            .map(|m| m.as_str().to_ascii_uppercase())
            .unwrap_or_else(|| default_authority.clone());
        return format!(
            "{authority} {} to {authority} {}",
            &caps[2].to_ascii_uppercase(),
            &caps[4].to_ascii_uppercase()
        );
    }

    let bare_ors_re = Regex::new(r"(?i)^\s*(\d{1,3}[A-Z]?\.\d{3}(?:\([^)]+\))?)\s*$").unwrap();
    if let Some(caps) = bare_ors_re.captures(&normalized) {
        return format!("{default_authority} {}", &caps[1].to_ascii_uppercase());
    }

    let utcr_re = Regex::new(r"(?i)\butcr\s+(\d{1,3}[A-Z]?\.\d{3}(?:\([^)]+\))?)").unwrap();
    normalized = utcr_re
        .replace_all(&normalized, |caps: &regex::Captures| {
            format!("UTCR {}", &caps[1].to_ascii_uppercase())
        })
        .to_string();

    let utcr_chapter_re = Regex::new(r"(?i)^\s*utcr\s+chapter\s+(\d{1,3}[A-Z]?)\s*$").unwrap();
    if let Some(caps) = utcr_chapter_re.captures(&normalized) {
        return format!("UTCR Chapter {}", &caps[1].to_ascii_uppercase());
    }

    let ors_re = Regex::new(r"(?i)\bors\s+(\d{1,3}[A-Z]?\.\d{3}(?:\([^)]+\))?)").unwrap();
    normalized = ors_re
        .replace_all(&normalized, |caps: &regex::Captures| {
            format!("ORS {}", &caps[1].to_ascii_uppercase())
        })
        .to_string();

    let ors_chapter_re = Regex::new(r"(?i)^\s*ors\s+chapter\s+(\d{1,3}[A-Z]?)\s*$").unwrap();
    if let Some(caps) = ors_chapter_re.captures(&normalized) {
        return format!("ORS Chapter {}", &caps[1].to_ascii_uppercase());
    }

    let chapter_re = Regex::new(r"(?i)^\s*chapter\s+(\d{1,3}[A-Z]?)\s*$").unwrap();
    if let Some(caps) = chapter_re.captures(&normalized) {
        if default_authority == "UTCR" {
            return format!("UTCR Chapter {}", &caps[1].to_ascii_uppercase());
        }
        return format!("Chapter {}", &caps[1].to_ascii_uppercase());
    }

    normalized
}

fn parse_query_citations(q: &str) -> Vec<QueryCitation> {
    let citation_re =
        Regex::new(r"(?i)\b(?:(ORS|UTCR)\s+)?(\d{1,3}[A-Z]?)\.(\d{3})((?:\([A-Za-z0-9]+\))*)")
            .unwrap();
    citation_re
        .captures_iter(q)
        .filter_map(|caps| {
            let raw = caps.get(0)?.as_str().trim().to_string();
            let authority_family = caps
                .get(1)
                .map(|m| m.as_str().to_ascii_uppercase())
                .unwrap_or_else(|| "ORS".to_string());
            let chapter = caps.get(2)?.as_str().to_ascii_uppercase();
            let section = caps.get(3)?.as_str().to_string();
            let subsection_text = caps.get(4).map(|m| m.as_str()).unwrap_or_default();
            let subsections = parse_subsections(subsection_text);
            let base = format!("{authority_family} {chapter}.{section}");
            let normalized = format!("{base}{subsection_text}");
            let parent = (!subsections.is_empty()).then(|| base.clone());

            Some(QueryCitation {
                raw,
                authority_family,
                normalized,
                base,
                chapter,
                section,
                subsections,
                parent,
            })
        })
        .collect()
}

fn parse_citation_ranges(q: &str) -> Vec<QueryCitationRange> {
    let range_re = Regex::new(
        r"(?i)\b(?:(ORS|UTCR)\s+)?(\d{1,3}[A-Z]?)\.(\d{3})(?:\([^)]+\))*\s+(?:to|through|thru|-|–)\s+(?:(ORS|UTCR)\s+)?(\d{1,3}[A-Z]?)\.(\d{3})(?:\([^)]+\))*",
    )
    .unwrap();
    range_re
        .captures_iter(q)
        .filter_map(|caps| {
            let start_authority = caps.get(1).map(|m| m.as_str().to_ascii_uppercase());
            let end_authority = caps.get(4).map(|m| m.as_str().to_ascii_uppercase());
            let authority_family = start_authority
                .clone()
                .or(end_authority.clone())
                .unwrap_or_else(|| "ORS".to_string());
            if let (Some(start), Some(end)) = (&start_authority, &end_authority) {
                if start != end {
                    return None;
                }
            }
            let start_chapter = caps.get(2)?.as_str().to_ascii_uppercase();
            let end_chapter = caps.get(5)?.as_str().to_ascii_uppercase();
            if start_chapter != end_chapter {
                return None;
            }
            let start = format!(
                "{} {}.{}",
                authority_family,
                start_chapter,
                caps.get(3)?.as_str()
            );
            let end = format!(
                "{} {}.{}",
                authority_family,
                end_chapter,
                caps.get(6)?.as_str()
            );
            Some(QueryCitationRange {
                raw: caps.get(0)?.as_str().trim().to_string(),
                authority_family,
                start,
                end,
                chapter: start_chapter,
            })
        })
        .collect()
}

fn parse_subsections(value: &str) -> Vec<String> {
    let subsection_re = Regex::new(r"\(([A-Za-z0-9]+)\)").unwrap();
    subsection_re
        .captures_iter(value)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

fn parse_chapter_query(q: &str) -> (Option<String>, Option<String>, Option<String>) {
    let chapter_re =
        Regex::new(r"(?i)^\s*(?:(ORS|UTCR)\s+)?chapter\s+(\d{1,3}[A-Z]?)\b\s*(.*)$").unwrap();
    if let Some(caps) = chapter_re.captures(q) {
        let authority_family = caps.get(1).map(|m| m.as_str().to_ascii_uppercase());
        let chapter = caps
            .get(2)
            .map(|m| m.as_str().to_ascii_uppercase())
            .unwrap_or_default();
        let residual = caps
            .get(3)
            .map(|m| clean_query_spacing(m.as_str()))
            .filter(|value| !value.is_empty());
        return (Some(chapter), authority_family, residual);
    }
    (None, None, None)
}

fn remove_ranges_from_query(q: &str, ranges: &[QueryCitationRange]) -> String {
    ranges.iter().fold(q.to_string(), |current, range| {
        current.replace(&range.raw, " ")
    })
}

fn remove_citations_from_query(q: &str, citations: &[QueryCitation]) -> String {
    citations.iter().fold(q.to_string(), |current, citation| {
        current.replace(&citation.raw, " ")
    })
}

fn candidate_matches_authority_family(candidate: &SearchResult, authority_family: &str) -> bool {
    let expected = normalized_authority_filter(Some(authority_family))
        .unwrap_or_else(|| authority_family.to_ascii_uppercase());
    candidate
        .authority_family
        .as_deref()
        .and_then(|value| normalized_authority_filter(Some(value)))
        .or_else(|| {
            candidate
                .citation
                .as_deref()
                .and_then(infer_authority_family_from_citation)
        })
        .map(|actual| actual == expected)
        .unwrap_or(false)
}

fn infer_authority_family_from_citation(citation: &str) -> Option<String> {
    let upper = citation.trim().to_ascii_uppercase();
    if upper.starts_with("UTCR ") {
        Some("UTCR".to_string())
    } else if upper.starts_with("ORS ") {
        Some("ORS".to_string())
    } else {
        None
    }
}

fn clean_residual_text(value: String) -> Option<String> {
    let cleaned = clean_query_spacing(&value);
    (!cleaned.is_empty()).then_some(cleaned)
}

fn clean_query_spacing(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|ch: char| matches!(ch, ',' | ';' | ':' | '-' | '–'))
        .trim()
        .to_string()
}

fn is_exact_candidate(candidate: &SearchResult) -> bool {
    candidate.rank_source.as_deref() == Some("exact")
        || candidate
            .score_breakdown
            .as_ref()
            .and_then(|breakdown| breakdown.exact)
            .unwrap_or(0.0)
            > 0.0
}

fn order_candidates_after_rerank(
    candidates: Vec<SearchResult>,
    positions: &HashMap<usize, (usize, f32)>,
) -> Vec<SearchResult> {
    let mut exact = Vec::new();
    let mut reranked_candidates = Vec::new();
    let mut remainder = Vec::new();

    for (idx, candidate) in candidates.into_iter().enumerate() {
        if is_exact_candidate(&candidate) {
            exact.push(candidate);
        } else if let Some((position, _)) = positions.get(&idx) {
            reranked_candidates.push((*position, candidate));
        } else {
            remainder.push(candidate);
        }
    }

    exact.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    reranked_candidates.sort_by_key(|(position, _)| *position);
    remainder.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut ordered = exact;
    ordered.extend(
        reranked_candidates
            .into_iter()
            .map(|(_, candidate)| candidate),
    );
    ordered.extend(remainder);
    ordered
}

fn detect_search_intent(q: &str) -> SearchIntent {
    let citation_re = Regex::new(r"^(?:ORS|UTCR)\s+\d{1,3}[A-Z]?\.\d{3}(?:\([^)]+\))?$").unwrap();
    let chapter_re = Regex::new(r"(?i)^(?:(?:ORS|UTCR)\s+)?chapter\s+\d{1,3}[A-Z]?$").unwrap();
    let definition_re = Regex::new(r"(?i)^definition\s+of|defines?\b|meaning\s+of").unwrap();
    let deadline_re =
        Regex::new(r"(?i)deadline|within\s+\d+\s+days|by\s+the\s+\w+\s+day|how long|when must")
            .unwrap();
    let penalty_re =
        Regex::new(r"(?i)penalty|fine|misdemeanor|felony|punished\s+by|civil penalty").unwrap();
    let notice_re = Regex::new(r"(?i)notice|notify|inform\b|written notice").unwrap();
    let actor_re =
        Regex::new(r"(?i)landlord|tenant|director|department|public\s+body|must .* do|shall")
            .unwrap();
    let history_re =
        Regex::new(r"(?i)operative|effective|amended|repealed|renumbered|session law|current")
            .unwrap();

    if citation_re.is_match(q) {
        SearchIntent::Citation
    } else if chapter_re.is_match(q) {
        SearchIntent::Chapter
    } else if definition_re.is_match(q) {
        SearchIntent::Definition
    } else if deadline_re.is_match(q) {
        SearchIntent::Deadline
    } else if penalty_re.is_match(q) {
        SearchIntent::Penalty
    } else if notice_re.is_match(q) {
        SearchIntent::Notice
    } else if actor_re.is_match(q) {
        SearchIntent::Actor
    } else if history_re.is_match(q) {
        SearchIntent::History
    } else {
        SearchIntent::General
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum SearchIntent {
    Citation,
    Chapter,
    Definition,
    Deadline,
    Penalty,
    Notice,
    Actor,
    History,
    General,
}

impl SearchIntent {
    fn as_str(self) -> &'static str {
        match self {
            Self::Citation => "citation",
            Self::Chapter => "chapter",
            Self::Definition => "definition",
            Self::Deadline => "deadline",
            Self::Penalty => "penalty",
            Self::Notice => "notice",
            Self::Actor => "actor",
            Self::History => "history",
            Self::General => "general",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterStage {
    Early,
    Late,
}

impl FilterStage {
    fn includes_early(self) -> bool {
        matches!(self, Self::Early)
    }

    fn includes_late(self) -> bool {
        matches!(self, Self::Late)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(id: &str, score: f32, rank_source: Option<&str>) -> SearchResult {
        SearchResult {
            id: id.to_string(),
            kind: "provision".to_string(),
            authority_family: Some("ORS".to_string()),
            authority_type: Some("statute".to_string()),
            corpus_id: Some("or:ors".to_string()),
            citation: Some(format!("ORS 90.{id}")),
            title: None,
            chapter: Some("90".to_string()),
            status: Some("active".to_string()),
            snippet: "sample legal text".to_string(),
            score,
            vector_score: None,
            fulltext_score: None,
            graph_score: None,
            rerank_score: None,
            pre_rerank_score: None,
            rank_source: rank_source.map(ToString::to_string),
            score_breakdown: rank_source.and_then(|source| {
                (source == "exact").then_some(ScoreBreakdown {
                    exact: Some(100.0),
                    keyword: None,
                    vector: None,
                    rerank: None,
                    graph: None,
                    authority: None,
                    expansion: None,
                    penalties: None,
                })
            }),
            semantic_types: Vec::new(),
            source_backed: true,
            qc_warnings: Vec::new(),
            href: format!("/statutes/ORS 90.{id}"),
            source: None,
            graph: None,
        }
    }

    #[test]
    fn normalizes_common_ors_and_chapter_queries() {
        assert_eq!(normalize_search_query("90.300"), "ORS 90.300");
        assert_eq!(normalize_search_query("ors 90.300(1)"), "ORS 90.300(1)");
        assert_eq!(
            normalize_search_query("utcr 2.010(4)(a)"),
            "UTCR 2.010(4)(a)"
        );
        assert_eq!(
            normalize_search_query_with_authority("2.010", Some("UTCR")),
            "UTCR 2.010"
        );
        assert_eq!(
            normalize_search_query("90.320 to 90.330"),
            "ORS 90.320 to ORS 90.330"
        );
        assert_eq!(normalize_search_query(" chapter 90 "), "Chapter 90");
        assert_eq!(
            normalize_search_query("“landlord” notice"),
            "\"landlord\" notice"
        );
    }

    #[test]
    fn detects_legal_search_intents() {
        assert_eq!(detect_search_intent("ORS 90.300"), SearchIntent::Citation);
        assert_eq!(detect_search_intent("UTCR 2.010"), SearchIntent::Citation);
        assert_eq!(detect_search_intent("Chapter 90"), SearchIntent::Chapter);
        assert_eq!(
            detect_search_intent("definition of dwelling unit"),
            SearchIntent::Definition
        );
        assert_eq!(
            detect_search_intent("security deposit deadline"),
            SearchIntent::Deadline
        );
        assert_eq!(detect_search_intent("civil penalty"), SearchIntent::Penalty);
    }

    #[test]
    fn auto_mode_routes_citation_like_queries_to_citation_mode() {
        let citation_plan = analyze_search_query("ORS 90.300");
        let chapter_topic_plan = analyze_search_query("chapter 90 habitability");
        let general_plan = analyze_search_query("security deposit deadline");

        assert_eq!(
            SearchService::resolve_mode(SearchMode::Auto, &citation_plan),
            SearchMode::Citation
        );
        assert_eq!(
            SearchService::resolve_mode(SearchMode::Auto, &chapter_topic_plan),
            SearchMode::Hybrid
        );
        assert_eq!(
            SearchService::resolve_mode(SearchMode::Auto, &general_plan),
            SearchMode::Hybrid
        );
    }

    #[test]
    fn analyzes_citations_ranges_and_chapter_topics() {
        let bare = analyze_search_query("90.300");
        assert_eq!(bare.analysis.normalized_query, "ORS 90.300");
        assert_eq!(bare.analysis.intent, "citation");
        assert_eq!(bare.analysis.citations[0].normalized, "ORS 90.300");

        let subsection = analyze_search_query("ORS 90.320(1)(a)");
        assert_eq!(subsection.analysis.citations[0].base, "ORS 90.320");
        assert_eq!(
            subsection.analysis.citations[0].subsections,
            vec!["1".to_string(), "a".to_string()]
        );
        assert_eq!(
            subsection.analysis.citations[0].parent.as_deref(),
            Some("ORS 90.320")
        );

        let range = analyze_search_query("ORS 90.320 to 90.330");
        assert_eq!(range.analysis.ranges[0].start, "ORS 90.320");
        assert_eq!(range.analysis.ranges[0].end, "ORS 90.330");
        assert_eq!(range.analysis.ranges[0].chapter, "90");

        let utcr = analyze_search_query("UTCR 2.010(4)(a)");
        assert_eq!(utcr.analysis.citations[0].authority_family, "UTCR");
        assert_eq!(utcr.analysis.citations[0].base, "UTCR 2.010");
        assert_eq!(
            utcr.analysis.citations[0].subsections,
            vec!["4".to_string(), "a".to_string()]
        );

        let utcr_range = analyze_search_query("UTCR 21.040 to 21.140");
        assert_eq!(utcr_range.analysis.ranges[0].authority_family, "UTCR");
        assert_eq!(utcr_range.analysis.ranges[0].start, "UTCR 21.040");
        assert_eq!(utcr_range.analysis.ranges[0].end, "UTCR 21.140");

        let chapter_topic = analyze_search_query("chapter 90 habitability");
        assert_eq!(
            chapter_topic.analysis.inferred_chapter.as_deref(),
            Some("90")
        );
        assert_eq!(
            chapter_topic.analysis.residual_text.as_deref(),
            Some("habitability")
        );
        assert_eq!(chapter_topic.chapter_filter.as_deref(), Some("90"));
        assert_eq!(chapter_topic.retrieval_query, "habitability");
    }

    #[test]
    fn retrieval_filters_record_public_filter_names_and_vector_chunk_hints() {
        let filters = SearchRetrievalFilters::from_query(&SearchQuery {
            q: "security deposit deadline".to_string(),
            r#type: Some("deadline".to_string()),
            authority_family: Some("ORS".to_string()),
            chapter: Some("90".to_string()),
            status: Some("all".to_string()),
            mode: Some(SearchMode::Hybrid),
            limit: None,
            offset: None,
            include: None,
            semantic_type: None,
            current_only: Some(true),
            source_backed: Some(true),
            has_citations: None,
            has_deadlines: Some(true),
            has_penalties: None,
            needs_review: None,
        });

        assert_eq!(filters.vector_chunk_type(), Some("deadline_block"));
        assert_eq!(
            filters.applied_filter_names(),
            vec![
                "type",
                "authority_family",
                "chapter",
                "current_only",
                "source_backed",
                "has_deadlines"
            ]
        );
    }

    #[test]
    fn authoritative_rerank_keeps_exact_first_then_reranked_then_remainder() {
        let candidates = vec![
            result("300", 100.0, Some("exact")),
            result("301", 9.0, Some("keyword")),
            result("302", 20.0, Some("keyword")),
            result("303", 50.0, Some("keyword")),
        ];
        let positions = HashMap::from([(2usize, (0usize, 0.91_f32)), (1usize, (1usize, 0.72_f32))]);

        let ordered = order_candidates_after_rerank(candidates, &positions);
        let ids = ordered
            .into_iter()
            .map(|result| result.id)
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["300", "302", "301", "303"]);
    }

    #[test]
    fn exact_marking_beats_graph_expanded_candidates() {
        let mut exact = result("300", 2.0, None);
        let mut expanded = result("301", 40.0, Some("keyword"));
        expanded.rank_source = Some("graph-expanded".to_string());

        mark_exact_result(&mut exact, 100.0, "exact");

        assert!(exact.score > expanded.score);
        assert_eq!(exact.rank_source.as_deref(), Some("exact"));
    }

    #[test]
    fn direct_response_uses_structured_match_type() {
        let response = direct_response_from_result(
            true,
            DirectMatchType::ExactProvision,
            "ORS 90.320(1)(a)".to_string(),
            result("320", 4.0, Some("exact")),
            None,
        );

        assert!(response.matched);
        assert_eq!(response.match_type, DirectMatchType::ExactProvision);
        assert_eq!(response.normalized_query, "ORS 90.320(1)(a)");
    }
}
