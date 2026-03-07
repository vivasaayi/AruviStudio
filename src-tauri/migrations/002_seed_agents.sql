-- Seed default agent definitions for AruviStudio MVP

-- Insert default agent definitions
INSERT INTO agent_definitions (id, name, role, description, prompt_template_ref, allowed_tools, boundaries, enabled) VALUES
('req-analysis-agent', 'Requirement Analysis Agent', 'requirement_analysis', 'Analyzes task requirements and clarifies ambiguities', 'req_analysis_v1', '["read_context","analyze_requirements"]', '{"max_tokens": 2000, "instructions": "Focus on clarifying requirements, identifying missing information, and suggesting improvements."}', 1),
('planning-agent', 'Planning Agent', 'planning', 'Creates detailed implementation plans for tasks', 'planning_v1', '["read_context","create_plan","analyze_codebase"]', '{"max_tokens": 3000, "instructions": "Create actionable implementation plans with specific steps, file changes, and testing approach."}', 1),
('coding-agent', 'Coding Agent', 'coding', 'Implements code changes according to approved plans', 'coding_v1', '["read_files","write_files","modify_code","create_files"]', '{"max_tokens": 4000, "instructions": "Implement code changes following the approved plan. Be specific about file paths and implementation details."}', 1),
('unit-test-agent', 'Unit Test Agent', 'unit_test_generation', 'Generates comprehensive unit tests', 'unit_test_v1', '["read_code","write_tests","analyze_coverage"]', '{"max_tokens": 2500, "instructions": "Generate unit tests covering happy paths, edge cases, and error conditions."}', 1),
('integration-test-agent', 'Integration Test Agent', 'integration_test_generation', 'Creates integration tests for component interactions', 'integration_test_v1', '["read_code","write_tests","analyze_dependencies"]', '{"max_tokens": 2500, "instructions": "Focus on testing interactions between components and data flow."}', 1),
('ui-test-agent', 'UI Test Planning Agent', 'ui_test_planning', 'Plans UI validation scenarios', 'ui_test_planning_v1', '["analyze_ui","plan_tests","document_scenarios"]', '{"max_tokens": 2000, "instructions": "Plan user interaction tests and expected UI behaviors."}', 1),
('qa-agent', 'QA Validation Agent', 'qa_validation', 'Reviews implementation and validates against requirements', 'qa_validation_v1', '["read_code","read_tests","validate_requirements","check_quality"]', '{"max_tokens": 2500, "instructions": "Validate that implementation meets acceptance criteria and identify any issues."}', 1),
('security-agent', 'Security Review Agent', 'security_review', 'Reviews code for security vulnerabilities', 'security_review_v1', '["analyze_security","check_vulnerabilities","review_patterns"]', '{"max_tokens": 2000, "instructions": "Check for common security issues, input validation, and secure coding practices."}', 1),
('performance-agent', 'Performance Review Agent', 'performance_review', 'Reviews implementation for performance considerations', 'performance_review_v1', '["analyze_performance","check_bottlenecks","suggest_optimizations"]', '{"max_tokens": 2000, "instructions": "Identify potential performance bottlenecks and suggest optimizations."}', 1);

-- Insert default model provider (LM Studio)
INSERT INTO model_providers (id, name, provider_type, base_url, enabled) VALUES
('lm-studio-default', 'LM Studio (Local)', 'openai_compatible', 'http://localhost:1234', 1);

-- Insert default model definition (DeepSeek)
INSERT INTO model_definitions (id, provider_id, name, context_window, capability_tags, enabled) VALUES
('deepseek-coder', 'lm-studio-default', 'deepseek-coder', 32768, '["coding","analysis","planning"]', 1);

-- Bind agents to the default model
INSERT INTO agent_model_bindings (id, agent_id, model_id, priority) VALUES
('req-analysis-binding', 'req-analysis-agent', 'deepseek-coder', 1),
('planning-binding', 'planning-agent', 'deepseek-coder', 1),
('coding-binding', 'coding-agent', 'deepseek-coder', 1),
('unit-test-binding', 'unit-test-agent', 'deepseek-coder', 1),
('integration-test-binding', 'integration-test-agent', 'deepseek-coder', 1),
('ui-test-binding', 'ui-test-agent', 'deepseek-coder', 1),
('qa-binding', 'qa-agent', 'deepseek-coder', 1),
('security-binding', 'security-agent', 'deepseek-coder', 1),
('performance-binding', 'performance-agent', 'deepseek-coder', 1);