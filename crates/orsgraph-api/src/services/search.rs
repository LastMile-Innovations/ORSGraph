use crate::error::ApiResult;
use crate::models::search::*;
use crate::services::graph_expand::GraphExpandService;
use crate::services::neo4j::Neo4jService;
use crate::services::rerank::RerankService;
use crate::services::vector_search::VectorSearchService;
use regex::Regex;
use std::sync::Arc;

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

    pub async fn search(&self, query: SearchQuery) -> ApiResult<SearchResponse> {
        let normalized = self.normalize_query(&query.q);
        let intent = self.detect_intent(&normalized);
        let requested_mode = query.mode.unwrap_or(SearchMode::Hybrid);
        let mode = if requested_mode == SearchMode::Auto {
            SearchMode::Hybrid
        } else {
            requested_mode
        };

        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        let offset = query.offset.unwrap_or(0);
        let candidate_limit = self
            .rerank
            .as_ref()
            .map(|r| r.candidates_limit())
            .unwrap_or(limit as usize * 4)
            .max(limit as usize)
            .min(250);

        let mut warnings = Vec::new();
        let mut retrieval = RetrievalInfo::default();
        let mut results = Vec::new();

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
                .search_fulltext(&normalized, query.r#type.as_deref(), candidate_limit as u32)
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
                match vector.search_chunks(&normalized, candidate_limit).await {
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
                                    &normalized,
                                    query.r#type.as_deref(),
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
                        .search_fulltext(
                            &normalized,
                            query.r#type.as_deref(),
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

        let mut candidates = self.rank_and_dedupe(results, intent);
        retrieval.graph_expanded_candidates =
            match self.graph_expand.expand_candidates(&mut candidates).await {
                Ok(count) => count,
                Err(e) => {
                    warnings.push(format!("Graph expansion failed: {}", e));
                    0
                }
            };
        let unfiltered_candidates = candidates.clone();
        self.apply_filters(&mut candidates, &query);

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

        let facets = Some(self.neo4j.aggregate_facets(&unfiltered_candidates));
        let mut rerank_info = None;

        if let Some(reranker) = &self.rerank {
            let candidate_count = candidates.len().min(reranker.candidates_limit());
            if candidate_count > 0 && mode != SearchMode::Citation {
                let rerank_slice = candidate_count;
                match reranker
                    .rerank(&normalized, &candidates[..rerank_slice])
                    .await
                {
                    Ok(output) => {
                        let mut rerank_map = std::collections::HashMap::new();
                        for res in &output.results {
                            rerank_map.insert(res.index, res.score);
                        }

                        for (idx, candidate) in candidates.iter_mut().take(rerank_slice).enumerate()
                        {
                            candidate.pre_rerank_score = Some(candidate.score);
                            if let Some(&rerank_score) = rerank_map.get(&idx) {
                                candidate.rerank_score = Some(rerank_score);
                                candidate.score = rerank_score * 5.0
                                    + self.calculate_legal_boosts(candidate, intent)
                                    + candidate.vector_score.unwrap_or(0.0)
                                    + candidate.fulltext_score.unwrap_or(0.0) * 0.75
                                    + candidate.graph_score.unwrap_or(0.0);
                                if candidate.rank_source.as_deref() != Some("exact") {
                                    candidate.rank_source = Some("rerank".to_string());
                                }
                                match &mut candidate.score_breakdown {
                                    Some(breakdown) => breakdown.rerank = Some(rerank_score),
                                    None => {
                                        candidate.score_breakdown = Some(ScoreBreakdown {
                                            exact: None,
                                            keyword: candidate.fulltext_score,
                                            vector: candidate.vector_score,
                                            rerank: Some(rerank_score),
                                            graph: candidate.graph_score,
                                            authority: None,
                                            penalties: None,
                                        });
                                    }
                                }
                            }
                        }

                        self.enforce_exact_priority(&mut candidates);
                        candidates.sort_by(|a, b| {
                            b.score
                                .partial_cmp(&a.score)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
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
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

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

    fn apply_filters(&self, candidates: &mut Vec<SearchResult>, query: &SearchQuery) {
        let result_type = query
            .r#type
            .as_deref()
            .filter(|value| !value.eq_ignore_ascii_case("all"));
        let chapter = query.chapter.as_deref().filter(|value| !value.is_empty());
        let status = query.status.as_deref().filter(|value| !value.is_empty());
        let semantic_type = query
            .semantic_type
            .as_deref()
            .filter(|value| !value.eq_ignore_ascii_case("all"));
        let current_only = query.current_only.unwrap_or(false);
        let source_backed_only = query.source_backed.unwrap_or(false);
        let has_citations = query.has_citations.unwrap_or(false);
        let has_deadlines = query.has_deadlines.unwrap_or(false);
        let has_penalties = query.has_penalties.unwrap_or(false);
        let needs_review = query.needs_review.unwrap_or(false);

        candidates.retain(|candidate| {
            if let Some(result_type) = result_type {
                if !self.matches_result_type(candidate, result_type) {
                    return false;
                }
            }
            if let Some(chapter) = chapter {
                if candidate.chapter.as_deref() != Some(chapter) {
                    return false;
                }
            }
            if let Some(status) = status {
                if status != "all" && candidate.status.as_deref() != Some(status) {
                    return false;
                }
            }
            if current_only {
                if candidate
                    .status
                    .as_deref()
                    .map(|value| value != "active")
                    .unwrap_or(false)
                {
                    return false;
                }
            }
            if source_backed_only && !candidate.source_backed {
                return false;
            }
            if has_citations && !self.has_citations(candidate) {
                return false;
            }
            if has_deadlines
                && !self.has_semantic_signal(candidate, &["deadline", "temporaleffect"])
            {
                return false;
            }
            if has_penalties && !self.has_semantic_signal(candidate, &["penalty"]) {
                return false;
            }
            if needs_review && candidate.qc_warnings.is_empty() {
                return false;
            }
            if let Some(semantic_type) = semantic_type {
                if !self.has_semantic_signal(candidate, &[semantic_type]) {
                    return false;
                }
            }
            true
        });
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
        candidate.rank_source.as_deref() == Some("exact")
            || candidate
                .score_breakdown
                .as_ref()
                .and_then(|breakdown| breakdown.exact)
                .unwrap_or(0.0)
                > 0.0
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

    fn detect_intent(&self, q: &str) -> SearchIntent {
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
        target.score = target.score.max(other.score) + 0.05;
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
