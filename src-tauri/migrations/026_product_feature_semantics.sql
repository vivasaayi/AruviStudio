-- Hard cutover from rollout/capability-slice terminology to Product Area > Capability > Feature.

UPDATE capabilities
SET node_kind = 'feature'
WHERE node_kind = 'rollout';

CREATE TABLE references_next (
    id TEXT PRIMARY KEY NOT NULL,
    scope_type TEXT NOT NULL CHECK(scope_type IN ('strategy_node','product','capability','feature','delivery_item')),
    scope_id TEXT NOT NULL,
    title TEXT NOT NULL,
    reference_kind TEXT NOT NULL DEFAULT 'note' CHECK(reference_kind IN ('note','external_doc','architecture','customer_evidence','regulatory','design_packet','standard','other')),
    uri TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO references_next (
    id,
    scope_type,
    scope_id,
    title,
    reference_kind,
    uri,
    content,
    created_at,
    updated_at
)
SELECT
    id,
    CASE
        WHEN scope_type = 'capability_slice' THEN 'feature'
        ELSE scope_type
    END,
    scope_id,
    title,
    reference_kind,
    uri,
    content,
    created_at,
    updated_at
FROM "references";

DROP TABLE "references";
ALTER TABLE references_next RENAME TO "references";
CREATE INDEX IF NOT EXISTS idx_references_scope ON "references"(scope_type, scope_id);
