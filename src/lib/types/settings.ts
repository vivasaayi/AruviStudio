export interface MigrationStatus {
  version: number;
  description: string;
  success: boolean;
  installed_on: string;
}

export interface DatabaseHealth {
  applied_migrations: number;
  latest_version: number | null;
  migrations: MigrationStatus[];
}

export interface MobileBridgeStatus {
  bind_host: string;
  bind_port: number;
  host_source: string;
  port_source: string;
  bind_scope: string;
  detected_lan_ip: string | null;
  desktop_base_url: string;
  phone_base_url: string | null;
  lan_ready: boolean;
  bind_changes_require_restart: boolean;
  env_overrides_settings: boolean;
  guidance: string;
}

export interface McpBridgeStatus {
  bind_host: string;
  bind_port: number;
  host_source: string;
  port_source: string;
  bind_scope: string;
  detected_lan_ip: string | null;
  desktop_base_url: string;
  lan_base_url: string | null;
  endpoint_url: string;
  lan_endpoint_url: string | null;
  token_configured: boolean;
  requests_allowed: boolean;
  auth_mode: string;
  origin_policy: string;
  bind_changes_require_restart: boolean;
  env_overrides_settings: boolean;
  guidance: string;
}
