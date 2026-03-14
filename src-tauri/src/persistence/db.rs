use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    repair_legacy_workflow_stage_history_fk(&pool).await?;

    Ok(pool)
}

async fn repair_legacy_workflow_stage_history_fk(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let schema: Option<String> = sqlx::query_scalar(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='workflow_stage_history'",
    )
    .fetch_optional(pool)
    .await?;

    let Some(schema_sql) = schema else {
        return Ok(());
    };

    if !schema_sql.contains("workflow_runs_legacy")
        && !schema_sql.contains("workflow_runs_fix_legacy")
    {
        return Ok(());
    }

    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE workflow_stage_history RENAME TO workflow_stage_history_legacy_fix")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE TABLE workflow_stage_history (
            id TEXT PRIMARY KEY NOT NULL,
            workflow_run_id TEXT NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
            from_stage TEXT NOT NULL,
            to_stage TEXT NOT NULL,
            trigger TEXT NOT NULL DEFAULT 'automatic',
            notes TEXT NOT NULL DEFAULT '',
            transitioned_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO workflow_stage_history (
            id, workflow_run_id, from_stage, to_stage, trigger, notes, transitioned_at
        )
        SELECT
            id, workflow_run_id, from_stage, to_stage, trigger, notes, transitioned_at
        FROM workflow_stage_history_legacy_fix",
    )
    .execute(pool)
    .await?;
    sqlx::query("DROP TABLE workflow_stage_history_legacy_fix")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_stage_history_workflow ON workflow_stage_history(workflow_run_id)")
        .execute(pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await?;

    Ok(())
}
