-- Add protocol selection without rebuilding provider tables or touching model
-- definitions and agent bindings, which reference them with cascading FKs.
ALTER TABLE model_providers ADD COLUMN protocol TEXT NOT NULL DEFAULT 'openai_compatible'
    CHECK(protocol IN ('openai_compatible', 'anthropic'))
    CHECK(protocol != 'anthropic' OR provider_type = 'openai_compatible');
