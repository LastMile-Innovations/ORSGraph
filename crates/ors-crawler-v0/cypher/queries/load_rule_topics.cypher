UNWIND $rows AS row
MERGE (topic:RuleTopic {rule_topic_id: row.rule_topic_id})
SET topic += row { .name, .normalized_name }
SET topic.id = row.rule_topic_id,
    topic.graph_kind = 'rule_topic',
    topic.schema_version = '1.0.0',
    topic.source_system = 'ors_crawler',
    topic.updated_at = datetime()
SET topic.created_at = coalesce(topic.created_at, datetime())
