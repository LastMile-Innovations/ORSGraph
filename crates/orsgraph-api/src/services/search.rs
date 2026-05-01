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

    fn resolve_mode(requested_mode: SearchMode, intent: SearchIntent) -> SearchMode {
        match (requested_mode, intent) {
            (SearchMode::Auto, SearchIntent::Citation | SearchIntent::Chapter) => {
                SearchMode::Citation
            }
            (SearchMode::Auto, _) => SearchMode::Hybrid,
            (mode, _) => mode,
        }
    }

    pub async fn search(&self, query: SearchQuery) -> ApiResult<SearchResponse> {
        let started_at = Instant::now();
        let normalized = self.normalize_query(&query.q);
        let intent = self.detect_intent(&normalized);
        let requested_mode = query.mode.unwrap_or_default();
        let mode = Self::resolve_mode(requested_mode, intent);
        let filters = SearchRetrievalFilters::from_query(&query);
        let applied_filters = filters.applied_filter_names();

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

        if normalized.is_empty() {
            return Ok(SearchResponse {
                query: query.q,
                normalized_query: normalized,
                intent: format!("{:?}", intent),
                mode,
                total: 0,
                limit,
                offset,
                results,
                facets: Some(self.neo4j.aggregate_facets(&[])),
                warnings,
                retrieval,
                embeddings: Some(EmbeddingsInfo {
                    enabled: self.vector.is_some(),
                    model: self.vector.as_ref().map(|v| v.model().to_string()),
                    profile: self.vector.as_ref().map(|v| v.profile().to_string()),
                    dimension: self.vector.as_ref().map(|v| v.dimension()),
                }),
                rerank: Some(RerankInfo {
                    enabled: self.rerank.is_some(),
                    model: self.rerank.as_ref().map(|r| r.model().to_string()),
                    candidate_count: None,
                    returned_count: None,
                    total_tokens: None,
                }),
                took_ms: started_at.elapsed().as_millis() as u64,
                applied_filters,
            });
        }

        let should_run_exact = matches!(mode, SearchMode::Citation | SearchMode::Hybrid)
            || matches!(intent, SearchIntent::Citation | SearchIntent::Chapter);
        let should_run_keyword = matches!(mode, SearchMode::Keyword | SearchMode::Hybrid);
        let should_run_vector = matches!(mode, SearchMode::Semantic | SearchMode::Hybrid);

        if should_run_exact {
            match self.neo4j.search_exact(&normalized).await {
                Ok(exact_results) => {
                    retrieval.exact_candidates = exact_results.len();
                    for mut res in exact_results {
                        res.rank_source = Some("exact".to_string());
                        res.score = 100.0 + res.score;
                        res.score_breakdown = Some(ScoreBreakdown {
                            exact: Some(100.0),
                            keyword: None,
                            vector: None,
                            rerank: None,
                            graph: None,
                            authority: None,
                            penalties: None,
                        });
                        results.push(res);
                    }
                }
                Err(e) => warnings.push(format!("Exact citation lookup failed: {}", e)),
            }
        }

        if should_run_keyword {
            match self
                .neo4j
                .search_fulltext(&normalized, &filters, candidate_limit as u32)
                .await
            {
                Ok(mut keyword_results) => {
                    retrieval.fulltext_candidates = keyword_results.len();
                    for res in &mut keyword_results {
                        if res.rank_source.is_none() {
                            res.rank_source = Some("keyword".to_string());
                        }
                    }
                    results.extend(keyword_results);
                }
                Err(e) => warnings.push(format!("Full-text search failed: {}", e)),
            }
        }

        if should_run_vector {
            if let Some(vector) = &self.vector {
                match vector
                    .search_chunks(&normalized, candidate_limit, &filters)
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
                                .search_fulltext(&normalized, &filters, candidate_limit as u32)
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
                        .search_fulltext(&normalized, &filters, candidate_limit as u32)
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

        let mut candidates = self.rank_and_dedupe(results, intent);
        self.apply_filters(&mut candidates, &filters, FilterStage::Early);
        retrieval.filtered_candidates = candidates.len();
        if candidates.len() > pre_expand_limit {
            candidates.truncate(pre_expand_limit);
        }
        retrieval.capped_candidates = candidates.len();

        retrieval.graph_expanded_candidates =
            match self.graph_expand.expand_candidates(&mut candidates).await {
                Ok(count) => count,
                Err(e) => {
                    warnings.push(format!("Graph expansion failed: {}", e));
                    0
                }
            };
        self.apply_filters(&mut candidates, &filters, FilterStage::Late);

        for candidate in &mut candidates {
            let boosts = self.calculate_legal_boosts(candidate, intent);
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
                match reranker
                    .rerank(&normalized, &candidates[..rerank_slice])
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
                            intent,
                        );
                        rerank_applied = !reranked.is_empty();
                        retrieval.reranked_candidates = output.results.len();

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
            rerank_info = Some(RerankInfo {
                enabled: self.rerank.is_some(),
                model: self.rerank.as_ref().map(|r| r.model().to_string()),
                candidate_count: None,
                returned_count: None,
                total_tokens: None,
            });
        }

        let total = candidates.len();
        let final_results: Vec<SearchResult> = candidates
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        Ok(SearchResponse {
            query: query.q.clone(),
            normalized_query: normalized,
            intent: format!("{:?}", intent),
            mode,
            total,
            limit,
            offset,
            results: final_results,
            facets,
            warnings,
            retrieval,
            embeddings: Some(EmbeddingsInfo {
                enabled: self.vector.is_some(),
                model: self.vector.as_ref().map(|v| v.model().to_string()),
                profile: self.vector.as_ref().map(|v| v.profile().to_string()),
                dimension: self.vector.as_ref().map(|v| v.dimension()),
            }),
            rerank: rerank_info,
            took_ms: started_at.elapsed().as_millis() as u64,
            applied_filters,
        })
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
                "semantic" => matches!(
                    kind.as_str(),
                    "legalsemanticnode"
                        | "obligation"
                        | "exception"
                        | "deadline"
                        | "penalty"
                        | "remedy"
                        | "requirednotice"
                ),
                "notice" | "requirednotice" => {
                    kind == "requirednotice" || self.has_semantic_signal(candidate, &["notice"])
                }
                "history" => matches!(
                    kind.as_str(),
                    "sourcenote" | "statusevent" | "temporaleffect" | "sessionlaw" | "amendment"
                ),
                "source_note" | "sourcenote" => kind == "sourcenote",
                "temporal_effect" | "temporaleffect" => kind == "temporaleffect",
                "taxrule" | "tax_rule" => kind == "taxrule",
                "moneyamount" | "money_amount" => kind == "moneyamount",
                "ratelimit" | "rate_limit" => kind == "ratelimit",
                "legalactor" | "legal_actor" | "actor" => kind == "legalactor",
                "legalaction" | "legal_action" | "action" => kind == "legalaction",
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

    pub async fn direct_open(&self, q: &str) -> ApiResult<DirectOpenResponse> {
        let normalized = self.normalize_query(q);
        let results = self.neo4j.search_exact(&normalized).await?;

        if let Some(first) = results.into_iter().next() {
            Ok(DirectOpenResponse {
                matched: true,
                kind: first.kind,
                citation: first.citation.unwrap_or_default(),
                canonical_id: first.id,
                href: first.href,
            })
        } else {
            Ok(DirectOpenResponse {
                matched: false,
                kind: "".to_string(),
                citation: q.to_string(),
                canonical_id: "".to_string(),
                href: "".to_string(),
            })
        }
    }

    pub async fn suggest(&self, q: &str, limit: Option<u32>) -> ApiResult<Vec<SuggestResult>> {
        let limit = limit.unwrap_or(10);
        self.neo4j.suggest(q, limit).await
    }

    fn normalize_query(&self, q: &str) -> String {
        normalize_search_query(q)
    }

    fn detect_intent(&self, q: &str) -> SearchIntent {
        detect_search_intent(q)
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
            "provision" => 10,
            "statute" | "legaltextidentity" | "legaltextversion" => 9,
            "definition" | "definedterm" => 8,
            "obligation" | "exception" | "deadline" | "penalty" | "remedy" | "requirednotice"
            | "taxrule" | "moneyamount" | "ratelimit" | "legalactor" | "legalaction" => 7,
            "sourcenote" | "sessionlaw" | "amendment" | "temporaleffect" => 6,
            "chunk" | "retrievalchunk" => 5,
            _ => 0,
        }
    }
}

fn normalize_search_query(q: &str) -> String {
    let mut normalized = q.trim().to_string();

    normalized = normalized
        .replace('“', "\"")
        .replace('”', "\"")
        .replace('‘', "'")
        .replace('’', "'");

    let bare_ors_re = Regex::new(r"(?i)^\s*(\d{1,3}\.\d{3}(?:\([^)]+\))?)\s*$").unwrap();
    if let Some(caps) = bare_ors_re.captures(&normalized) {
        return format!("ORS {}", &caps[1]);
    }

    let ors_re = Regex::new(r"(?i)\bors\s+(\d{1,3}\.\d{3}(?:\([^)]+\))?)").unwrap();
    normalized = ors_re
        .replace_all(&normalized, |caps: &regex::Captures| {
            format!("ORS {}", &caps[1])
        })
        .to_string();

    let chapter_re = Regex::new(r"(?i)^\s*chapter\s+(\d{1,3})\s*$").unwrap();
    if let Some(caps) = chapter_re.captures(&normalized) {
        return format!("Chapter {}", &caps[1]);
    }

    normalized
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
    let ors_re = Regex::new(r"^ORS\s+\d{1,3}\.\d{3}(?:\([^)]+\))?$").unwrap();
    let chapter_re = Regex::new(r"(?i)^chapter\s+\d{1,3}$").unwrap();
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

    if ors_re.is_match(q) {
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
        assert_eq!(normalize_search_query(" chapter 90 "), "Chapter 90");
        assert_eq!(
            normalize_search_query("“landlord” notice"),
            "\"landlord\" notice"
        );
    }

    #[test]
    fn detects_legal_search_intents() {
        assert_eq!(detect_search_intent("ORS 90.300"), SearchIntent::Citation);
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
        assert_eq!(
            SearchService::resolve_mode(SearchMode::Auto, SearchIntent::Citation),
            SearchMode::Citation
        );
        assert_eq!(
            SearchService::resolve_mode(SearchMode::Auto, SearchIntent::General),
            SearchMode::Hybrid
        );
    }

    #[test]
    fn retrieval_filters_record_public_filter_names_and_vector_chunk_hints() {
        let filters = SearchRetrievalFilters::from_query(&SearchQuery {
            q: "security deposit deadline".to_string(),
            r#type: Some("deadline".to_string()),
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
}
