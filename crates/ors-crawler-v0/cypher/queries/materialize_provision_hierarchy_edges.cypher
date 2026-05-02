// Create hierarchical relationships between provisions
// Creates HAS_PARENT, CONTAINS, NEXT, and PREVIOUS edges to establish the provision hierarchy and ordering

// Parent-child relationships - using indexed local_path and version_id
CALL {
    MATCH (child:Provision)
    WHERE size(child.local_path) > 1 AND NOT child.local_path = ['root']
    WITH child, child.local_path[..-1] AS parentPath
    MATCH (parent:Provision {version_id: child.version_id, local_path: parentPath})
    MERGE (child)-[:HAS_PARENT]->(parent)
    MERGE (parent)-[:CONTAINS]->(child)
} IN TRANSACTIONS OF 5000 ROWS;

// Sequential relationships (NEXT/PREVIOUS) - using indexed order_index within same version
CALL {
    MATCH (current:Provision)
    MATCH (next:Provision {version_id: current.version_id, order_index: current.order_index + 1})
    MERGE (current)-[:NEXT]->(next)
    MERGE (next)-[:PREVIOUS]->(current)
} IN TRANSACTIONS OF 5000 ROWS;
