-- Existing bindings are preserved; block changes that would create new
-- local Whisper bindings for text-completion agents.
CREATE TRIGGER prevent_local_speech_agent_binding_insert
BEFORE INSERT ON agent_model_bindings
WHEN EXISTS (
    SELECT 1 FROM model_definitions m JOIN model_providers p ON p.id=m.provider_id
    WHERE m.id=NEW.model_id AND p.provider_type='local_runtime'
)
BEGIN
    SELECT RAISE(ABORT, 'Local Whisper models cannot be used for agent text completion');
END;

CREATE TRIGGER prevent_local_speech_agent_binding_update
BEFORE UPDATE OF model_id ON agent_model_bindings
WHEN NEW.model_id != OLD.model_id AND EXISTS (
    SELECT 1 FROM model_definitions m JOIN model_providers p ON p.id=m.provider_id
    WHERE m.id=NEW.model_id AND p.provider_type='local_runtime'
)
BEGIN
    SELECT RAISE(ABORT, 'Local Whisper models cannot be used for agent text completion');
END;

CREATE TRIGGER prevent_bound_model_becoming_local_speech
BEFORE UPDATE OF provider_id ON model_definitions
WHEN NEW.provider_id != OLD.provider_id
    AND EXISTS (SELECT 1 FROM agent_model_bindings b WHERE b.model_id=OLD.id)
    AND EXISTS (SELECT 1 FROM model_providers p WHERE p.id=NEW.provider_id AND p.provider_type='local_runtime')
BEGIN
    SELECT RAISE(ABORT, 'Bound agent models cannot use a local Whisper provider');
END;

CREATE TRIGGER prevent_bound_provider_becoming_local_speech
BEFORE UPDATE OF provider_type ON model_providers
WHEN NEW.provider_type='local_runtime' AND OLD.provider_type!='local_runtime'
    AND EXISTS (
        SELECT 1 FROM model_definitions m JOIN agent_model_bindings b ON b.model_id=m.id
        WHERE m.provider_id=OLD.id
    )
BEGIN
    SELECT RAISE(ABORT, 'Providers with bound agent models cannot become local Whisper providers');
END;
