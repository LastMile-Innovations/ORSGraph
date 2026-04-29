// Check Neo4j version and vector support
CALL dbms.components() YIELD name, versions, edition
RETURN name, versions, edition;

// Check if vector functions exist (2025.10+)
SHOW FUNCTIONS YIELD name, signature
WHERE name STARTS WITH 'vector'
RETURN name, signature
LIMIT 10;

// Check if vector indexes can be created (5.13+)
SHOW INDEXES YIELD name, type
WHERE type = 'VECTOR'
RETURN count(*) as vector_index_count;

// Check current database format
SHOW DATABASES YIELD name, currentStatus, storeFormat
RETURN name, currentStatus, storeFormat;
