use crate::auth::AuthVerifier;
use crate::config::ApiConfig;
use crate::services::admin::AdminService;
use crate::services::analytics::AnalyticsService;
use crate::services::casebuilder::{
    AssemblyAiProviderConfig, CaseBuilderService, CaseBuilderServiceConfig,
};
use crate::services::embedding::EmbeddingService;
use crate::services::graph_expand::GraphExpandService;
use crate::services::health::HealthService;
use crate::services::home::HomeService;
use crate::services::neo4j::Neo4jService;
use crate::services::object_store::object_store_from_config;
use crate::services::rerank::RerankService;
use crate::services::rules::RuleApplicabilityResolver;
use crate::services::search::SearchService;
use crate::services::stats::StatsService;
use crate::services::vector_search::VectorSearchService;
use neo4rs::Graph;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub neo4j: Arc<Graph>,
    pub neo4j_service: Arc<Neo4jService>,
    pub search_service: Arc<SearchService>,
    pub embedding_service: Option<Arc<EmbeddingService>>,
    pub vector_search_service: Option<Arc<VectorSearchService>>,
    pub graph_expand_service: Arc<GraphExpandService>,
    pub rerank_service: Option<Arc<RerankService>>,
    pub stats_service: Arc<StatsService>,
    pub health_service: Arc<HealthService>,
    pub admin_service: Arc<AdminService>,
    pub analytics_service: Arc<AnalyticsService>,
    pub home_service: Arc<HomeService>,
    pub casebuilder_service: Arc<CaseBuilderService>,
    pub rule_applicability_resolver: Arc<RuleApplicabilityResolver>,
    pub auth_verifier: Option<Arc<AuthVerifier>>,
    pub config: Arc<ApiConfig>,
}

impl AppState {
    pub async fn new(config: ApiConfig) -> anyhow::Result<Self> {
        let neo4j = Arc::new(
            Graph::new(
                config.neo4j_uri.clone(),
                config.neo4j_user.clone(),
                config.neo4j_password.clone(),
            )
            .await?,
        );

        let neo4j_service = Arc::new(Neo4jService::new(neo4j.clone()));

        // Ensure indexes exist
        if let Err(e) = neo4j_service.ensure_indexes().await {
            tracing::error!("Failed to ensure Neo4j indexes: {}", e);
        }

        let rerank_service = if config.rerank_enabled && config.voyage_api_key.is_some() {
            Some(Arc::new(RerankService::new(
                config.voyage_api_key.clone().unwrap(),
                config.rerank_model.clone(),
                config.rerank_candidates,
                config.rerank_top_k,
                config.rerank_max_doc_tokens,
                config.rerank_timeout_ms,
            )))
        } else {
            None
        };

        let embedding_service = if (config.vector_enabled || config.vector_search_enabled)
            && config.voyage_api_key.is_some()
        {
            Some(Arc::new(EmbeddingService::new(
                config.voyage_api_key.clone().unwrap(),
                config.embedding_model.clone(),
                config.vector_dimension,
                config.rerank_timeout_ms,
            )))
        } else {
            None
        };

        let vector_search_service = embedding_service.as_ref().map(|embeddings| {
            Arc::new(VectorSearchService::new(
                neo4j_service.clone(),
                embeddings.clone(),
                config.vector_index.clone(),
                config.vector_top_k,
                config.vector_min_score,
                config.vector_profile.clone(),
            ))
        });

        let graph_expand_service = Arc::new(GraphExpandService::new(neo4j_service.clone()));

        let search_service = Arc::new(SearchService::new(
            neo4j_service.clone(),
            vector_search_service.clone(),
            graph_expand_service.clone(),
            rerank_service.clone(),
        ));

        let stats_service = Arc::new(StatsService::new(neo4j_service.clone()));
        let health_service = Arc::new(HealthService::new(neo4j_service.clone()));
        let auth_verifier = AuthVerifier::from_config(&config)?.map(Arc::new);
        let config = Arc::new(config);
        let admin_service = Arc::new(AdminService::new(config.clone()).await?);
        let analytics_service = Arc::new(AnalyticsService::new(neo4j_service.clone()));
        let home_service = Arc::new(HomeService::new(
            stats_service.clone(),
            health_service.clone(),
            analytics_service.clone(),
        ));
        let rule_applicability_resolver = Arc::new(RuleApplicabilityResolver::new(neo4j.clone()));
        let object_store = object_store_from_config(&config).await?;
        let casebuilder_service = Arc::new(CaseBuilderService::new(CaseBuilderServiceConfig {
            neo4j: neo4j_service.clone(),
            object_store,
            upload_ttl_seconds: config.r2_upload_ttl_seconds,
            download_ttl_seconds: config.r2_download_ttl_seconds,
            max_upload_bytes: config.r2_max_upload_bytes,
            ast_entity_inline_bytes: config.casebuilder_ast_entity_inline_bytes,
            ast_snapshot_inline_bytes: config.casebuilder_ast_snapshot_inline_bytes,
            ast_block_inline_bytes: config.casebuilder_ast_block_inline_bytes,
            assemblyai: AssemblyAiProviderConfig {
                enabled: config.assemblyai_enabled,
                api_key: config.assemblyai_api_key.clone(),
                base_url: config.assemblyai_base_url.clone(),
                webhook_url: config.assemblyai_webhook_url.clone(),
                webhook_secret: config.assemblyai_webhook_secret.clone(),
                timeout_ms: config.assemblyai_timeout_ms,
                max_media_bytes: config.assemblyai_max_media_bytes,
            },
        }));

        if let Err(e) = casebuilder_service.ensure_indexes().await {
            tracing::error!("Failed to ensure CaseBuilder indexes: {}", e);
        }

        Ok(Self {
            neo4j,
            neo4j_service,
            search_service,
            embedding_service,
            vector_search_service,
            graph_expand_service,
            rerank_service,
            stats_service,
            health_service,
            analytics_service,
            home_service,
            casebuilder_service,
            rule_applicability_resolver,
            admin_service,
            auth_verifier,
            config,
        })
    }
}
