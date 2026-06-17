CREATE TABLE IF NOT EXISTS bulk_import_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    source_path TEXT NOT NULL,
    import_format TEXT NOT NULL CHECK(import_format IN ('json','csv')),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','running','completed','failed')),
    total_records INTEGER NOT NULL DEFAULT 0,
    processed_records INTEGER NOT NULL DEFAULT 0,
    product_count INTEGER NOT NULL DEFAULT 0,
    product_area_count INTEGER NOT NULL DEFAULT 0,
    capability_count INTEGER NOT NULL DEFAULT 0,
    feature_count INTEGER NOT NULL DEFAULT 0,
    work_item_count INTEGER NOT NULL DEFAULT 0,
    failed_records INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    started_at TEXT,
    completed_at TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_bulk_import_jobs_status ON bulk_import_jobs(status);
CREATE INDEX IF NOT EXISTS idx_bulk_import_jobs_created_at ON bulk_import_jobs(created_at);

CREATE TABLE IF NOT EXISTS bulk_import_job_errors (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL REFERENCES bulk_import_jobs(id) ON DELETE CASCADE,
    row_index INTEGER,
    record_type TEXT NOT NULL DEFAULT '',
    record_id TEXT NOT NULL DEFAULT '',
    message TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_bulk_import_job_errors_job ON bulk_import_job_errors(job_id);
