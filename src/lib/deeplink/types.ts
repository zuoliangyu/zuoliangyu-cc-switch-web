import type { AppId } from "@/lib/api";

export type DeepLinkResource = "provider" | "prompt" | "mcp" | "skill";

export interface DeepLinkImportRequest {
  version: string;
  resource: DeepLinkResource;
  app?: AppId;
  name?: string;
  enabled?: boolean;
  homepage?: string;
  endpoint?: string;
  apiKey?: string;
  icon?: string;
  model?: string;
  notes?: string;
  haikuModel?: string;
  sonnetModel?: string;
  opusModel?: string;
  content?: string;
  description?: string;
  apps?: string;
  repo?: string;
  directory?: string;
  branch?: string;
  config?: string;
  configFormat?: string;
  configUrl?: string;
  usageEnabled?: boolean;
  usageScript?: string;
  usageApiKey?: string;
  usageBaseUrl?: string;
  usageAccessToken?: string;
  usageUserId?: string;
  usageAutoInterval?: number;
}

export interface McpImportError {
  id: string;
  error: string;
}

export interface McpImportSummary {
  importedCount: number;
  importedIds: string[];
  failed: McpImportError[];
}

export type DeepLinkImportResult =
  | { type: "provider"; id: string }
  | { type: "prompt"; id: string }
  | ({ type: "mcp" } & McpImportSummary)
  | { type: "skill"; key: string };
