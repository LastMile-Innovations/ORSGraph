// materialize_obligation_edges.cypher
// Materialize IMPOSED_ON, REQUIRES_ACTION, HAS_DEADLINE, etc.

CALL {
    MATCH (o:Obligation)
    WHERE o.actor_id IS NOT NULL
    MATCH (a:LegalActor {actor_id: o.actor_id})
    MERGE (o)-[:IMPOSED_ON]->(a)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (o:Obligation)
    WHERE o.action_id IS NOT NULL
    MATCH (act:LegalAction {action_id: o.action_id})
    MERGE (o)-[:REQUIRES_ACTION]->(act)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (o:Obligation)
    WHERE o.deadline_id IS NOT NULL
    MATCH (d:Deadline {deadline_id: o.deadline_id})
    MERGE (o)-[:HAS_DEADLINE]->(d)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (o:Obligation)
    WHERE o.exception_id IS NOT NULL
    MATCH (e:Exception {exception_id: o.exception_id})
    MERGE (o)-[:SUBJECT_TO]->(e)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (o:Obligation)
    WHERE o.penalty_id IS NOT NULL
    MATCH (pnl:Penalty {penalty_id: o.penalty_id})
    MERGE (o)-[:VIOLATION_PENALIZED_BY]->(pnl)
} IN TRANSACTIONS OF 5000 ROWS;

// Cross-links for exceptions/deadlines/penalties to source provision if not already handled
CALL {
    MATCH (e:Exception)
    WHERE e.target_obligation_id IS NOT NULL
    MATCH (o:Obligation {obligation_id: e.target_obligation_id})
    MERGE (e)-[:EXCEPTION_TO]->(o)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (d:Deadline)
    WHERE d.obligation_id IS NOT NULL
    MATCH (o:Obligation {obligation_id: d.obligation_id})
    MERGE (d)-[:APPLIES_TO]->(o)
} IN TRANSACTIONS OF 5000 ROWS;

CALL {
    MATCH (pnl:Penalty)
    WHERE pnl.obligation_id IS NOT NULL
    MATCH (o:Obligation {obligation_id: pnl.obligation_id})
    MERGE (pnl)-[:PENALIZES_VIOLATION_OF]->(o)
} IN TRANSACTIONS OF 5000 ROWS;
