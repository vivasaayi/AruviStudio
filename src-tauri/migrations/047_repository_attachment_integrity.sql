-- A repository may be attached to a scope once. Re-attaching it makes it the
-- default instead of creating a second ambiguous attachment.

DELETE FROM repository_attachments
WHERE rowid NOT IN (
    SELECT MIN(rowid)
    FROM repository_attachments
    GROUP BY scope_type, scope_id, repository_id
);

-- Keep the most recently created default for each scope, then guarantee that
-- scope has a single default attachment.
UPDATE repository_attachments
SET is_default = 0
WHERE is_default = 1
  AND rowid NOT IN (
      SELECT MAX(rowid)
      FROM repository_attachments
      WHERE is_default = 1
      GROUP BY scope_type, scope_id
  );

CREATE UNIQUE INDEX IF NOT EXISTS idx_repo_attachments_scope_repository
    ON repository_attachments(scope_type, scope_id, repository_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_repo_attachments_one_default
    ON repository_attachments(scope_type, scope_id)
    WHERE is_default = 1;
