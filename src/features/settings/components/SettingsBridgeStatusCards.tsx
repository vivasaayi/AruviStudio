import type { McpBridgeStatus, MobileBridgeStatus } from "../../../lib/types";
import { styles } from "../lib/settingsPageStyles";

type BridgeStatusCardProps = {
  onCopy: (value: string) => void;
};

type McpBridgeStatusCardProps = BridgeStatusCardProps & {
  status: McpBridgeStatus | undefined;
};

type MobileBridgeStatusCardProps = BridgeStatusCardProps & {
  status: MobileBridgeStatus | undefined;
};

export function McpBridgeStatusCard({ status, onCopy }: McpBridgeStatusCardProps) {
  return (
    <div style={styles.healthCard}>
      <div style={{ ...styles.label, marginBottom: 8 }}>Embedded MCP Status</div>
      {status ? (
        <>
          <div style={styles.healthGrid}>
            <div>
              <div style={styles.healthLabel}>Bind Scope</div>
              <div style={styles.healthValue}>{status.bind_scope}</div>
            </div>
            <div>
              <div style={styles.healthLabel}>Auth Mode</div>
              <div style={styles.healthValue}>{status.auth_mode}</div>
            </div>
          </div>
          <div style={{ ...styles.desc, marginBottom: 8 }}>
            {status.guidance}
          </div>
          <div style={{ marginBottom: 10 }}>
            <div style={styles.healthLabel}>Local MCP Endpoint</div>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 6 }}>
              <div style={{ ...styles.codeBox, flex: 1 }}>{status.endpoint_url}</div>
              <button style={{ ...styles.btn, marginLeft: 0 }} onClick={() => onCopy(status.endpoint_url)}>Copy</button>
            </div>
          </div>
          <div style={{ marginBottom: 10 }}>
            <div style={styles.healthLabel}>LAN MCP Endpoint</div>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 6 }}>
              <div style={{ ...styles.codeBox, flex: 1 }}>{status.lan_endpoint_url ?? "Keep bind host on 127.0.0.1 for local-only agent access, or switch to 0.0.0.0 and restart for same-LAN clients."}</div>
              {status.lan_endpoint_url && <button style={{ ...styles.btn, marginLeft: 0 }} onClick={() => onCopy(status.lan_endpoint_url!)}>Copy</button>}
            </div>
          </div>
          <div style={styles.desc}>
            Token configured: {status.token_configured ? "yes" : "no"}. Requests allowed: {status.requests_allowed ? "yes" : "no"}. Origin policy: {status.origin_policy} {status.env_overrides_settings ? "Environment variables currently override these settings. " : ""}{status.bind_changes_require_restart ? "Restart AruviStudio after changing bind host or port." : ""}
          </div>
        </>
      ) : (
        <div style={styles.desc}>Loading MCP bridge status...</div>
      )}
    </div>
  );
}

export function MobileBridgeStatusCard({ status, onCopy }: MobileBridgeStatusCardProps) {
  return (
    <div style={styles.healthCard}>
      <div style={{ ...styles.label, marginBottom: 8 }}>LAN Ready Status</div>
      {status ? (
        <>
          <div style={styles.healthGrid}>
            <div>
              <div style={styles.healthLabel}>Bind Scope</div>
              <div style={styles.healthValue}>{status.bind_scope}</div>
            </div>
            <div>
              <div style={styles.healthLabel}>Detected Mac LAN IP</div>
              <div style={styles.healthValue}>{status.detected_lan_ip ?? "Unavailable"}</div>
            </div>
          </div>
          <div style={{ ...styles.desc, marginBottom: 8 }}>
            {status.guidance}
          </div>
          <div style={{ marginBottom: 10 }}>
            <div style={styles.healthLabel}>Desktop Base URL</div>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 6 }}>
              <div style={{ ...styles.codeBox, flex: 1 }}>{status.desktop_base_url}</div>
              <button style={{ ...styles.btn, marginLeft: 0 }} onClick={() => onCopy(status.desktop_base_url)}>Copy</button>
            </div>
          </div>
          <div style={{ marginBottom: 10 }}>
            <div style={styles.healthLabel}>Phone Base URL</div>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 6 }}>
              <div style={{ ...styles.codeBox, flex: 1 }}>{status.phone_base_url ?? "Set bind host to 0.0.0.0 and restart to enable same-LAN access."}</div>
              {status.phone_base_url && <button style={{ ...styles.btn, marginLeft: 0 }} onClick={() => onCopy(status.phone_base_url!)}>Copy</button>}
            </div>
          </div>
          <div style={styles.desc}>
            Bind host source: {status.host_source}. Port source: {status.port_source}. {status.env_overrides_settings ? "Environment variables currently override these settings. " : ""}{status.bind_changes_require_restart ? "Restart AruviStudio after changing bind host or port." : ""}
          </div>
        </>
      ) : (
        <div style={styles.desc}>Loading mobile bridge status...</div>
      )}
    </div>
  );
}
