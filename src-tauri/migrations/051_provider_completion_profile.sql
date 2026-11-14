-- Existing providers keep their current OpenAI-compatible request shape.
ALTER TABLE model_providers ADD COLUMN completion_profile TEXT NOT NULL DEFAULT 'standard'
    CHECK(completion_profile IN ('standard', 'openai_reasoning'));
