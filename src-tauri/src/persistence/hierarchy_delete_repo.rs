use crate::error::AppError;
use sqlx::{Sqlite, SqlitePool, Transaction};

// Hierarchy deletes preserve delivery records and references, while removing only
// hierarchy-owned assignments. The caller's delete and all reconciliation are atomic.
async fn detach_scope(tx: &mut Transaction<'_, Sqlite>, product_id: &str, scope: &str, id: &str) -> Result<(), AppError> {
    let reference_scope = if scope == "product_area" { "product_area" } else { "capability" };
    sqlx::query("UPDATE \"references\" SET scope_type='product',scope_id=?,updated_at=datetime('now') WHERE scope_id=? AND (scope_type=? OR (?='capability' AND scope_type='feature'))")
        .bind(product_id).bind(id).bind(reference_scope).bind(scope).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM team_assignments WHERE scope_type=? AND scope_id=?")
        .bind(scope).bind(id).execute(&mut **tx).await?;
    sqlx::query("UPDATE work_items SET source_node_id=NULL,source_node_type=NULL,updated_at=datetime('now') WHERE source_node_id=? AND (source_node_type=? OR (?='capability' AND source_node_type='feature'))")
        .bind(id).bind(scope).bind(scope).execute(&mut **tx).await?;
    if scope == "product_area" {
        sqlx::query("UPDATE work_items SET product_area_id=NULL,updated_at=datetime('now') WHERE product_area_id=?").bind(id).execute(&mut **tx).await?;
        sqlx::query("DELETE FROM repository_attachments WHERE scope_type='product_area' AND scope_id=?").bind(id).execute(&mut **tx).await?;
    } else {
        sqlx::query("UPDATE work_items SET capability_id=NULL,updated_at=datetime('now') WHERE capability_id=?").bind(id).execute(&mut **tx).await?;
    }
    Ok(())
}

pub async fn delete_hierarchy_node(pool: &SqlitePool, scope: &str, id: &str, expected: Option<(&str, &str)>) -> Result<bool, AppError> {
    let mut tx = pool.begin().await?;
    let deleted = delete_hierarchy_node_in_transaction(&mut tx, scope, id, expected).await?;
    tx.commit().await?;
    Ok(deleted)
}

pub(crate) async fn delete_hierarchy_node_in_transaction(tx: &mut Transaction<'_, Sqlite>, scope: &str, id: &str, expected: Option<(&str, &str)>) -> Result<bool, AppError> {
    let lock_sql = match scope {
        "product_area" => "UPDATE product_areas SET name=name WHERE id=?",
        "capability" => "UPDATE capabilities SET name=name WHERE id=?",
        _ => return Err(AppError::Validation("Unsupported hierarchy node type".into())),
    };
    if sqlx::query(lock_sql).bind(id).execute(&mut **tx).await?.rows_affected() == 0 { return Ok(false); }
    let lookup = if scope == "product_area" {
        "SELECT product_id,name FROM product_areas WHERE id=?"
    } else {
        "SELECT a.product_id,c.name FROM capabilities c JOIN product_areas a ON a.id=c.product_area_id WHERE c.id=?"
    };
    let (product_id, name): (String, String) = sqlx::query_as(lookup).bind(id).fetch_one(&mut **tx).await?;
    if let Some((expected_product, expected_name)) = expected {
        if product_id != expected_product || name != expected_name { return Ok(false); }
    }
    let ids: Vec<String> = if scope == "product_area" {
        sqlx::query_scalar("SELECT id FROM capabilities WHERE product_area_id=? ORDER BY level DESC,id")
            .bind(id).fetch_all(&mut **tx).await?
    } else {
        sqlx::query_scalar("WITH RECURSIVE descendants(id) AS (SELECT id FROM capabilities WHERE id=? UNION SELECT c.id FROM capabilities c JOIN descendants d ON c.parent_capability_id=d.id) SELECT id FROM capabilities WHERE id IN (SELECT id FROM descendants) ORDER BY level DESC,id")
            .bind(id).fetch_all(&mut **tx).await?
    };
    for capability_id in ids {
        detach_scope(tx, &product_id, "capability", &capability_id).await?;
        sqlx::query("DELETE FROM capabilities WHERE id=?").bind(&capability_id).execute(&mut **tx).await?;
    }
    if scope == "product_area" {
        detach_scope(tx, &product_id, scope, id).await?;
        sqlx::query("DELETE FROM product_areas WHERE id=?").bind(id).execute(&mut **tx).await?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::delete_hierarchy_node;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn deletion_rolls_back_all_related_changes_on_failure() {
        let db = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        for sql in [
            "PRAGMA foreign_keys=ON",
            "CREATE TABLE product_areas(id TEXT PRIMARY KEY,product_id TEXT NOT NULL,name TEXT NOT NULL)",
            "CREATE TABLE capabilities(id TEXT PRIMARY KEY,product_area_id TEXT REFERENCES product_areas(id) ON DELETE CASCADE,parent_capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,level INTEGER NOT NULL,name TEXT NOT NULL)",
            "CREATE TABLE \"references\"(id TEXT PRIMARY KEY,scope_type TEXT NOT NULL,scope_id TEXT NOT NULL,updated_at TEXT)",
            "CREATE TABLE team_assignments(id TEXT PRIMARY KEY,scope_type TEXT NOT NULL,scope_id TEXT NOT NULL)",
            "CREATE TABLE repository_attachments(id TEXT PRIMARY KEY,scope_type TEXT NOT NULL,scope_id TEXT NOT NULL)",
            "CREATE TABLE work_items(id TEXT PRIMARY KEY,product_area_id TEXT REFERENCES product_areas(id) ON DELETE SET NULL,capability_id TEXT REFERENCES capabilities(id) ON DELETE SET NULL,source_node_id TEXT,source_node_type TEXT,updated_at TEXT)",
            "INSERT INTO product_areas VALUES('area','product','Area')",
            "INSERT INTO capabilities VALUES('parent','area',NULL,0,'Parent'),('child','area','parent',1,'Child')",
            "INSERT INTO \"references\" VALUES('ref','feature','child',NULL)",
            "INSERT INTO team_assignments VALUES('assignment','capability','child')",
            "INSERT INTO repository_attachments VALUES('attachment','product_area','area')",
            "INSERT INTO work_items VALUES('work','area','child','child','capability',NULL)",
            "CREATE TRIGGER reject_parent_delete BEFORE DELETE ON capabilities WHEN OLD.id='parent' BEGIN SELECT RAISE(ABORT,'test failure'); END",
        ] { sqlx::query(sql).execute(&db).await.unwrap(); }
        assert!(delete_hierarchy_node(&db, "product_area", "area", None).await.is_err());
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM capabilities").fetch_one(&db).await.unwrap();
        assert_eq!(count, 2);
        let scope: (String, String) = sqlx::query_as("SELECT scope_type,scope_id FROM \"references\" WHERE id='ref'").fetch_one(&db).await.unwrap();
        assert_eq!(scope, ("feature".into(), "child".into()));
        let work: (Option<String>, Option<String>, Option<String>) = sqlx::query_as("SELECT product_area_id,capability_id,source_node_id FROM work_items WHERE id='work'").fetch_one(&db).await.unwrap();
        assert_eq!(work, (Some("area".into()), Some("child".into()), Some("child".into())));
        let assignments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM team_assignments").fetch_one(&db).await.unwrap();
        let attachments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM repository_attachments").fetch_one(&db).await.unwrap();
        assert_eq!((assignments, attachments), (1, 1));
        sqlx::query("DROP TRIGGER reject_parent_delete").execute(&db).await.unwrap();
        assert!(delete_hierarchy_node(&db, "product_area", "area", Some(("product", "Area"))).await.unwrap());
        let scope: (String, String) = sqlx::query_as("SELECT scope_type,scope_id FROM \"references\" WHERE id='ref'").fetch_one(&db).await.unwrap();
        assert_eq!(scope, ("product".into(), "product".into()));
        let work: (Option<String>, Option<String>, Option<String>) = sqlx::query_as("SELECT product_area_id,capability_id,source_node_id FROM work_items WHERE id='work'").fetch_one(&db).await.unwrap();
        assert_eq!(work, (None, None, None));
    }
}
