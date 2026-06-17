-- Normalize legacy/imported catalog node-kind labels to the current
-- Product Area > Capability > Feature vocabulary.

UPDATE modules
SET node_kind = 'area'
WHERE lower(replace(node_kind, '-', '_')) IN (
    'area',
    'product_area',
    'module',
    'strategic_area',
    'domain',
    'subdomain',
    'capability',
    'feature_set',
    'feature_group'
);

UPDATE capabilities
SET node_kind = 'capability'
WHERE lower(replace(node_kind, '-', '_')) IN (
    'capability',
    'area',
    'product_area',
    'module',
    'strategic_area',
    'domain',
    'subdomain',
    'feature_set',
    'feature_group'
);

UPDATE capabilities
SET node_kind = 'feature'
WHERE lower(replace(node_kind, '-', '_')) IN (
    'feature',
    'rollout',
    'capability_slice'
);
