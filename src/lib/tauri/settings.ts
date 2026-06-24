import { invoke } from "./core";
import type {
  DatabaseHealth,
  McpBridgeStatus,
  MobileBridgeStatus,
} from "../types";

// Settings commands
export const getSetting = (key: string) => invoke<string | null>("get_setting", { key });
export const setSetting = (key: string, value: string) => invoke("set_setting", { key, value });
export const getMobileBridgeStatus = () => invoke<MobileBridgeStatus>("get_mobile_bridge_status");
export const getMcpBridgeStatus = () => invoke<McpBridgeStatus>("get_mcp_bridge_status");
export const getDatabaseHealth = () => invoke<DatabaseHealth>("get_database_health");
export const getActiveDatabasePath = () => invoke<string>("get_active_database_path");
export const getDatabasePathOverride = () => invoke<string | null>("get_database_path_override");
export const setDatabasePathOverride = (dbPath: string) =>
  invoke<void>("set_database_path_override", { dbPath, db_path: dbPath });
export const clearDatabasePathOverride = () => invoke<void>("clear_database_path_override");
