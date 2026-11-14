-- Opaque optimistic-concurrency token; never expose or hash the secret itself.
ALTER TABLE model_providers ADD COLUMN credential_revision TEXT NOT NULL DEFAULT '';
UPDATE model_providers SET credential_revision=lower(hex(randomblob(16)));
CREATE TRIGGER model_provider_credential_revision_insert
AFTER INSERT ON model_providers
BEGIN
    UPDATE model_providers SET credential_revision=lower(hex(randomblob(16))) WHERE id=NEW.id;
END;
CREATE TRIGGER model_provider_credential_revision
AFTER UPDATE OF auth_secret_ref ON model_providers
WHEN OLD.auth_secret_ref IS NOT NEW.auth_secret_ref
BEGIN
    UPDATE model_providers SET credential_revision=lower(hex(randomblob(16))) WHERE id=NEW.id;
END;
