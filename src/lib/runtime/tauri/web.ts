import type { Settings } from "@/types";
import type { Provider } from "@/types";
import type { McpServer, McpServersMap } from "@/types";
import type {
  OpenClawAgentsDefaults,
  OpenClawDefaultModel,
  OpenClawEnvConfig,
  OpenClawHealthWarning,
  OpenClawModelCatalogEntry,
  OpenClawToolsConfig,
  OpenClawWriteOutcome,
} from "@/types";
import type { SessionMessage, SessionMeta } from "@/types";
import type { AppId } from "@/lib/api";
import type { Prompt } from "@/lib/api";
import type {
  OptimizerConfig,
  RectifierConfig,
} from "@/lib/api/settings";
import type { DeleteSessionOptions, DeleteSessionResult } from "@/lib/api/sessions";
import type {
  DiscoverableSkill,
  SkillArchiveInstallResult,
  ImportSkillSelection,
  InstalledSkill,
  SkillBackupEntry,
  SkillRepo,
  SkillUninstallResult,
  UnmanagedSkill,
} from "@/lib/api/skills";
import type { SwitchResult } from "@/lib/api/providers";
import type {
  AppProxyConfig,
  CircuitBreakerConfig,
  CircuitBreakerStats,
  FailoverQueueItem,
  GlobalProxyConfig,
  ProviderHealth,
  ProxyConfig,
  ProxyServerInfo,
  ProxyStatus,
  ProxyTakeoverStatus,
} from "@/types/proxy";
import type {
  DailyMemoryFileInfo,
  DailyMemorySearchResult,
} from "@/lib/api/workspace";
import type {
  DailyStats,
  LogFilters,
  ModelPricing,
  ModelStats,
  PaginatedLogs,
  ProviderLimitStatus,
  ProviderStats,
  RequestLog,
  UsageSummary,
} from "@/types/usage";
import {
  getDefaultAppProxyConfig,
  getDefaultGlobalProxyConfig,
  getDefaultProxyConfig,
  getDefaultProxyStatus,
  getDefaultProxyTakeoverStatus,
  getDefaultSettings,
} from "./defaults";

interface ProvidersResponse {
  providers: Record<string, Provider>;
  currentProviderId: string;
}

const DEFAULT_WEB_API_BASE = "http://127.0.0.1:8788";

export const getWebApiBase = (): string => {
  const configured = import.meta.env.VITE_LOCAL_API_BASE?.trim();
  return configured && configured.length > 0
    ? configured.replace(/\/+$/, "")
    : DEFAULT_WEB_API_BASE;
};

async function getErrorMessage(
  response: Response,
  fallback: string,
): Promise<string> {
  try {
    const contentType = response.headers.get("content-type") || "";
    if (contentType.includes("application/json")) {
      const payload = (await response.json()) as {
        error?: string;
        message?: string;
        detail?: string;
      };
      const detail = payload.error || payload.message || payload.detail;
      if (detail && detail.trim()) {
        return detail;
      }
    } else {
      const text = await response.text();
      if (text.trim()) {
        return text.trim();
      }
    }
  } catch (error) {
    console.warn("[runtime:web] failed to parse error response", error);
  }

  return fallback;
}

async function requestJson<T>(path: string): Promise<T> {
  const response = await fetch(`${getWebApiBase()}${path}`, {
    headers: {
      Accept: "application/json",
    },
  });

  if (!response.ok) {
    const fallback = `HTTP ${response.status} for ${path}`;
    throw new Error(await getErrorMessage(response, fallback));
  }

  return (await response.json()) as T;
}

async function requestWithBody<T>(
  path: string,
  method: "POST" | "PUT" | "DELETE",
  body?: unknown,
): Promise<T> {
  const response = await fetch(`${getWebApiBase()}${path}`, {
    method,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  if (!response.ok) {
    const fallback = `HTTP ${response.status} for ${method} ${path}`;
    throw new Error(await getErrorMessage(response, fallback));
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

async function requestFormData<T>(path: string, formData: FormData): Promise<T> {
  const response = await fetch(`${getWebApiBase()}${path}`, {
    method: "POST",
    headers: {
      Accept: "application/json",
    },
    body: formData,
  });

  if (!response.ok) {
    const fallback = `HTTP ${response.status} for POST ${path}`;
    throw new Error(await getErrorMessage(response, fallback));
  }

  return (await response.json()) as T;
}

export async function getWebSettings(): Promise<Settings> {
  try {
    return await requestJson<Settings>("/api/settings");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load settings from local service",
      error,
    );
    return getDefaultSettings();
  }
}

export async function getWebProviders(
  appId: AppId,
): Promise<ProvidersResponse> {
  try {
    return await requestJson<ProvidersResponse>(`/api/providers/${appId}`);
  } catch (error) {
    console.warn(
      `[runtime:web] failed to load providers for ${appId} from local service`,
      error,
    );
    return {
      providers: {},
      currentProviderId: "",
    };
  }
}

export async function getWebLiveProviderIds(appId: AppId): Promise<string[]> {
  try {
    return await requestJson<string[]>(
      `/api/providers/${appId}/live-provider-ids`,
    );
  } catch (error) {
    console.warn(
      `[runtime:web] failed to load live provider ids for ${appId} from local service`,
      error,
    );
    return [];
  }
}

export async function importWebProvidersFromLive(appId: AppId): Promise<number> {
  return requestWithBody<number>(`/api/providers/${appId}/import-live`, "POST");
}

export async function removeWebProviderFromLiveConfig(
  appId: AppId,
  id: string,
): Promise<boolean> {
  return requestWithBody<boolean>(
    `/api/providers/${appId}/live-config/${encodeURIComponent(id)}`,
    "DELETE",
  );
}

export async function getWebProxyStatus(): Promise<ProxyStatus> {
  try {
    return await requestJson<ProxyStatus>("/api/proxy/status");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load proxy status from local service",
      error,
    );
    return getDefaultProxyStatus();
  }
}

export async function getWebProxyTakeoverStatus(): Promise<ProxyTakeoverStatus> {
  try {
    return await requestJson<ProxyTakeoverStatus>("/api/proxy/takeover-status");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load proxy takeover status from local service",
      error,
    );
    return getDefaultProxyTakeoverStatus();
  }
}

export async function getWebProxyConfig(): Promise<ProxyConfig> {
  try {
    return await requestJson<ProxyConfig>("/api/proxy/config");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load proxy config from local service",
      error,
    );
    return getDefaultProxyConfig();
  }
}

export async function getWebGlobalProxyConfig(): Promise<GlobalProxyConfig> {
  try {
    return await requestJson<GlobalProxyConfig>("/api/proxy/global-config");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load global proxy config from local service",
      error,
    );
    return getDefaultGlobalProxyConfig();
  }
}

export async function getWebProxyConfigForApp(
  appId: AppId,
): Promise<AppProxyConfig> {
  try {
    return await requestJson<AppProxyConfig>(`/api/proxy/apps/${appId}/config`);
  } catch (error) {
    console.warn(
      `[runtime:web] failed to load proxy config for ${appId} from local service`,
      error,
    );
    return getDefaultAppProxyConfig(appId);
  }
}

export async function getWebIsProxyRunning(): Promise<boolean> {
  try {
    return await requestJson<boolean>("/api/proxy/running");
  } catch (error) {
    console.warn("[runtime:web] failed to load proxy running state", error);
    return false;
  }
}

export async function getWebIsLiveTakeoverActive(): Promise<boolean> {
  try {
    return await requestJson<boolean>("/api/proxy/live-takeover-active");
  } catch (error) {
    console.warn("[runtime:web] failed to load live takeover state", error);
    return false;
  }
}

export async function startWebProxyServer(): Promise<ProxyServerInfo> {
  return requestWithBody<ProxyServerInfo>("/api/proxy/start", "POST");
}

export async function stopWebProxyWithRestore(): Promise<void> {
  return requestWithBody<void>("/api/proxy/stop-with-restore", "POST");
}

export async function setWebProxyTakeoverForApp(
  appId: AppId,
  enabled: boolean,
): Promise<void> {
  return requestWithBody<void>(`/api/proxy/apps/${appId}/takeover`, "PUT", {
    enabled,
  });
}

export async function switchWebProxyProvider(
  appId: AppId,
  providerId: string,
): Promise<void> {
  return requestWithBody<void>(
    `/api/proxy/apps/${appId}/providers/${providerId}/switch`,
    "POST",
  );
}

export async function updateWebProxyConfig(config: ProxyConfig): Promise<void> {
  return requestWithBody<void>("/api/proxy/config", "PUT", config);
}

export async function updateWebGlobalProxyConfig(
  config: GlobalProxyConfig,
): Promise<void> {
  return requestWithBody<void>("/api/proxy/global-config", "PUT", config);
}

export async function updateWebProxyConfigForApp(
  config: AppProxyConfig,
): Promise<void> {
  return requestWithBody<void>(
    `/api/proxy/apps/${config.appType}/config`,
    "PUT",
    config,
  );
}

export async function getWebDefaultCostMultiplier(
  appId: AppId,
): Promise<string> {
  return requestJson<string>(
    `/api/proxy/apps/${appId}/default-cost-multiplier`,
  );
}

export async function setWebDefaultCostMultiplier(
  appId: AppId,
  value: string,
): Promise<void> {
  return requestWithBody<void>(
    `/api/proxy/apps/${appId}/default-cost-multiplier`,
    "PUT",
    { value },
  );
}

export async function getWebPricingModelSource(appId: AppId): Promise<string> {
  return requestJson<string>(`/api/proxy/apps/${appId}/pricing-model-source`);
}

export async function setWebPricingModelSource(
  appId: AppId,
  value: string,
): Promise<void> {
  return requestWithBody<void>(
    `/api/proxy/apps/${appId}/pricing-model-source`,
    "PUT",
    { value },
  );
}

export async function getWebProviderHealth(
  appId: AppId,
  providerId: string,
): Promise<ProviderHealth> {
  try {
    return await requestJson<ProviderHealth>(
      `/api/failover/apps/${appId}/providers/${providerId}/health`,
    );
  } catch (error) {
    console.warn(
      `[runtime:web] failed to load provider health for ${appId}/${providerId}`,
      error,
    );
    return {
      provider_id: providerId,
      app_type: appId,
      is_healthy: true,
      consecutive_failures: 0,
      last_success_at: null,
      last_failure_at: null,
      last_error: null,
      updated_at: new Date().toISOString(),
    };
  }
}

export async function getWebCircuitBreakerConfig(): Promise<CircuitBreakerConfig> {
  try {
    return await requestJson<CircuitBreakerConfig>(
      "/api/failover/circuit-breaker-config",
    );
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load circuit breaker config from local service",
      error,
    );
    return {
      failureThreshold: 4,
      successThreshold: 2,
      timeoutSeconds: 60,
      errorRateThreshold: 0.6,
      minRequests: 10,
    };
  }
}

export async function updateWebCircuitBreakerConfig(
  config: CircuitBreakerConfig,
): Promise<void> {
  return requestWithBody<void>(
    "/api/failover/circuit-breaker-config",
    "PUT",
    config,
  );
}

export async function getWebCircuitBreakerStats(
  appId: AppId,
  providerId: string,
): Promise<CircuitBreakerStats | null> {
  try {
    return await requestJson<CircuitBreakerStats | null>(
      `/api/failover/apps/${appId}/providers/${providerId}/circuit-breaker-stats`,
    );
  } catch (error) {
    console.warn(
      `[runtime:web] failed to load circuit breaker stats for ${appId}/${providerId}`,
      error,
    );
    return null;
  }
}

export async function getWebFailoverQueue(
  appId: AppId,
): Promise<FailoverQueueItem[]> {
  try {
    return await requestJson<FailoverQueueItem[]>(
      `/api/failover/apps/${appId}/queue`,
    );
  } catch (error) {
    console.warn(
      `[runtime:web] failed to load failover queue for ${appId} from local service`,
      error,
    );
    return [];
  }
}

export async function getWebAvailableProvidersForFailover(
  appId: AppId,
): Promise<Provider[]> {
  try {
    return await requestJson<Provider[]>(
      `/api/failover/apps/${appId}/available-providers`,
    );
  } catch (error) {
    console.warn(
      `[runtime:web] failed to load available failover providers for ${appId}`,
      error,
    );
    return [];
  }
}

export async function addWebProviderToFailoverQueue(
  appId: AppId,
  providerId: string,
): Promise<void> {
  return requestWithBody<void>(`/api/failover/apps/${appId}/queue`, "POST", {
    providerId,
  });
}

export async function removeWebProviderFromFailoverQueue(
  appId: AppId,
  providerId: string,
): Promise<void> {
  return requestWithBody<void>(
    `/api/failover/apps/${appId}/queue/${providerId}`,
    "DELETE",
  );
}

export async function getWebAutoFailoverEnabled(
  appId: AppId,
): Promise<boolean> {
  try {
    return await requestJson<boolean>(
      `/api/failover/apps/${appId}/auto-enabled`,
    );
  } catch (error) {
    console.warn(
      `[runtime:web] failed to load auto failover flag for ${appId}`,
      error,
    );
    return false;
  }
}

export async function setWebAutoFailoverEnabled(
  appId: AppId,
  enabled: boolean,
): Promise<void> {
  return requestWithBody<void>(
    `/api/failover/apps/${appId}/auto-enabled`,
    "PUT",
    { enabled },
  );
}

export async function saveWebSettings(settings: Settings): Promise<boolean> {
  return requestWithBody<boolean>("/api/settings", "PUT", settings);
}

export async function getWebRectifierConfig(): Promise<RectifierConfig> {
  return requestJson<RectifierConfig>("/api/settings/rectifier");
}

export async function setWebRectifierConfig(
  config: RectifierConfig,
): Promise<boolean> {
  return requestWithBody<boolean>("/api/settings/rectifier", "PUT", config);
}

export async function getWebOptimizerConfig(): Promise<OptimizerConfig> {
  return requestJson<OptimizerConfig>("/api/settings/optimizer");
}

export async function setWebOptimizerConfig(
  config: OptimizerConfig,
): Promise<boolean> {
  return requestWithBody<boolean>("/api/settings/optimizer", "PUT", config);
}

export async function addWebProvider(
  appId: AppId,
  provider: Provider,
): Promise<boolean> {
  return requestWithBody<boolean>(`/api/providers/${appId}`, "POST", provider);
}

export async function updateWebProvider(
  appId: AppId,
  provider: Provider,
): Promise<boolean> {
  return requestWithBody<boolean>(
    `/api/providers/${appId}/${provider.id}`,
    "PUT",
    provider,
  );
}

export async function deleteWebProvider(
  appId: AppId,
  id: string,
): Promise<boolean> {
  return requestWithBody<boolean>(`/api/providers/${appId}/${id}`, "DELETE");
}

export async function switchWebProvider(
  appId: AppId,
  id: string,
): Promise<SwitchResult> {
  return requestWithBody<SwitchResult>(
    `/api/providers/${appId}/${id}/switch`,
    "POST",
  );
}

export async function getWebMcpServers(): Promise<McpServersMap> {
  try {
    return await requestJson<McpServersMap>("/api/mcp/servers");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load mcp servers from local service",
      error,
    );
    return {};
  }
}

export async function upsertWebMcpServer(server: McpServer): Promise<void> {
  return requestWithBody<void>("/api/mcp/servers", "POST", server);
}

export async function deleteWebMcpServer(id: string): Promise<boolean> {
  return requestWithBody<boolean>(`/api/mcp/servers/${id}`, "DELETE");
}

export async function toggleWebMcpApp(
  serverId: string,
  appId: AppId,
  enabled: boolean,
): Promise<void> {
  return requestWithBody<void>(
    `/api/mcp/servers/${serverId}/apps/${appId}`,
    "PUT",
    {
      enabled,
    },
  );
}

export async function importWebMcpFromApps(): Promise<number> {
  return requestWithBody<number>("/api/mcp/servers/import", "POST");
}

export async function getWebPrompts(
  appId: AppId,
): Promise<Record<string, Prompt>> {
  try {
    return await requestJson<Record<string, Prompt>>(`/api/prompts/${appId}`);
  } catch (error) {
    console.warn(
      `[runtime:web] failed to load prompts for ${appId} from local service`,
      error,
    );
    return {};
  }
}

export async function upsertWebPrompt(
  appId: AppId,
  id: string,
  prompt: Prompt,
): Promise<void> {
  return requestWithBody<void>(`/api/prompts/${appId}/${id}`, "PUT", prompt);
}

export async function deleteWebPrompt(appId: AppId, id: string): Promise<void> {
  return requestWithBody<void>(`/api/prompts/${appId}/${id}`, "DELETE");
}

export async function enableWebPrompt(appId: AppId, id: string): Promise<void> {
  return requestWithBody<void>(`/api/prompts/${appId}/${id}/enable`, "POST");
}

export async function importWebPromptFromFile(appId: AppId): Promise<string> {
  return requestWithBody<string>(`/api/prompts/${appId}/import`, "POST");
}

export async function getWebCurrentPromptFileContent(
  appId: AppId,
): Promise<string | null> {
  try {
    return await requestJson<string | null>(
      `/api/prompts/${appId}/current-file`,
    );
  } catch (error) {
    console.warn(
      `[runtime:web] failed to load current prompt file for ${appId} from local service`,
      error,
    );
    return null;
  }
}

export async function getWebInstalledSkills(): Promise<InstalledSkill[]> {
  try {
    return await requestJson<InstalledSkill[]>("/api/skills/installed");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load installed skills from local service",
      error,
    );
    return [];
  }
}

export async function getWebSkillBackups(): Promise<SkillBackupEntry[]> {
  try {
    return await requestJson<SkillBackupEntry[]>("/api/skills/backups");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load skill backups from local service",
      error,
    );
    return [];
  }
}

export async function uninstallWebSkillUnified(
  id: string,
): Promise<SkillUninstallResult> {
  return requestWithBody<SkillUninstallResult>(`/api/skills/${id}`, "DELETE");
}

export async function toggleWebSkillApp(
  id: string,
  app: AppId,
  enabled: boolean,
): Promise<boolean> {
  return requestWithBody<boolean>(`/api/skills/${id}/apps/${app}`, "PUT", {
    enabled,
  });
}

export async function getWebUnmanagedSkills(): Promise<UnmanagedSkill[]> {
  try {
    return await requestJson<UnmanagedSkill[]>("/api/skills/unmanaged");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to scan unmanaged skills from local service",
      error,
    );
    return [];
  }
}

export async function importWebSkillsFromApps(
  imports: ImportSkillSelection[],
): Promise<InstalledSkill[]> {
  return requestWithBody<InstalledSkill[]>(
    "/api/skills/import",
    "POST",
    imports,
  );
}

export async function getWebSkillRepos(): Promise<SkillRepo[]> {
  try {
    return await requestJson<SkillRepo[]>("/api/skills/repos");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load skill repos from local service",
      error,
    );
    return [];
  }
}

export async function addWebSkillRepo(repo: SkillRepo): Promise<boolean> {
  return requestWithBody<boolean>("/api/skills/repos", "POST", repo);
}

export async function removeWebSkillRepo(
  owner: string,
  name: string,
): Promise<boolean> {
  return requestWithBody<boolean>(
    `/api/skills/repos/${owner}/${name}`,
    "DELETE",
  );
}

export async function discoverWebAvailableSkills(): Promise<
  DiscoverableSkill[]
> {
  try {
    return await requestJson<DiscoverableSkill[]>("/api/skills/discover");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to discover skills from local service",
      error,
    );
    return [];
  }
}

export async function installWebSkillUnified(
  skill: DiscoverableSkill,
  currentApp: AppId,
): Promise<InstalledSkill> {
  return requestWithBody<InstalledSkill>("/api/skills/install", "POST", {
    skill,
    currentApp,
  });
}

export async function deleteWebSkillBackup(backupId: string): Promise<boolean> {
  return requestWithBody<boolean>(`/api/skills/backups/${backupId}`, "DELETE");
}

export async function restoreWebSkillBackup(
  backupId: string,
  currentApp: AppId,
): Promise<InstalledSkill> {
  return requestWithBody<InstalledSkill>(
    `/api/skills/backups/${backupId}`,
    "POST",
    {
      currentApp,
    },
  );
}

export async function installWebSkillArchives(
  files: File[],
  currentApp: AppId,
): Promise<SkillArchiveInstallResult[]> {
  const formData = new FormData();
  formData.append("currentApp", currentApp);
  files.forEach((file) => {
    formData.append("archives", file, file.name);
  });
  return requestFormData<SkillArchiveInstallResult[]>(
    "/api/skills/install-archives",
    formData,
  );
}

export async function getWebWorkspaceFile(
  filename: string,
): Promise<string | null> {
  return requestJson<string | null>(
    `/api/workspace/files/${encodeURIComponent(filename)}`,
  );
}

export async function saveWebWorkspaceFile(
  filename: string,
  content: string,
): Promise<void> {
  return requestWithBody<void>(
    `/api/workspace/files/${encodeURIComponent(filename)}`,
    "PUT",
    { content },
  );
}

export async function listWebDailyMemoryFiles(): Promise<DailyMemoryFileInfo[]> {
  return requestJson<DailyMemoryFileInfo[]>("/api/workspace/daily-memory");
}

export async function getWebDailyMemoryFile(
  filename: string,
): Promise<string | null> {
  return requestJson<string | null>(
    `/api/workspace/daily-memory/${encodeURIComponent(filename)}`,
  );
}

export async function saveWebDailyMemoryFile(
  filename: string,
  content: string,
): Promise<void> {
  return requestWithBody<void>(
    `/api/workspace/daily-memory/${encodeURIComponent(filename)}`,
    "PUT",
    { content },
  );
}

export async function deleteWebDailyMemoryFile(filename: string): Promise<void> {
  return requestWithBody<void>(
    `/api/workspace/daily-memory/${encodeURIComponent(filename)}`,
    "DELETE",
  );
}

export async function searchWebDailyMemoryFiles(
  query: string,
): Promise<DailyMemorySearchResult[]> {
  return requestJson<DailyMemorySearchResult[]>(
    `/api/workspace/daily-memory/search?query=${encodeURIComponent(query)}`,
  );
}

export async function getWebWorkspaceDirectoryPath(
  subdir: "workspace" | "memory",
): Promise<string> {
  return requestJson<string>(
    `/api/workspace/directories/${encodeURIComponent(subdir)}/path`,
  );
}

export async function getWebOpenClawDefaultModel(): Promise<OpenClawDefaultModel | null> {
  try {
    return await requestJson<OpenClawDefaultModel | null>(
      "/api/openclaw/default-model",
    );
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load openclaw default model from local service",
      error,
    );
    return null;
  }
}

export async function setWebOpenClawDefaultModel(
  model: OpenClawDefaultModel,
): Promise<OpenClawWriteOutcome> {
  return requestWithBody<OpenClawWriteOutcome>(
    "/api/openclaw/default-model",
    "PUT",
    model,
  );
}

export async function getWebOpenClawModelCatalog(): Promise<Record<
  string,
  OpenClawModelCatalogEntry
> | null> {
  try {
    return await requestJson<Record<string, OpenClawModelCatalogEntry> | null>(
      "/api/openclaw/model-catalog",
    );
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load openclaw model catalog from local service",
      error,
    );
    return null;
  }
}

export async function setWebOpenClawModelCatalog(
  catalog: Record<string, OpenClawModelCatalogEntry>,
): Promise<OpenClawWriteOutcome> {
  return requestWithBody<OpenClawWriteOutcome>(
    "/api/openclaw/model-catalog",
    "PUT",
    catalog,
  );
}

export async function getWebOpenClawAgentsDefaults(): Promise<OpenClawAgentsDefaults | null> {
  try {
    return await requestJson<OpenClawAgentsDefaults | null>(
      "/api/openclaw/agents-defaults",
    );
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load openclaw agents defaults from local service",
      error,
    );
    return null;
  }
}

export async function setWebOpenClawAgentsDefaults(
  defaults: OpenClawAgentsDefaults,
): Promise<OpenClawWriteOutcome> {
  return requestWithBody<OpenClawWriteOutcome>(
    "/api/openclaw/agents-defaults",
    "PUT",
    defaults,
  );
}

export async function getWebOpenClawEnv(): Promise<OpenClawEnvConfig> {
  try {
    return await requestJson<OpenClawEnvConfig>("/api/openclaw/env");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load openclaw env config from local service",
      error,
    );
    return {};
  }
}

export async function setWebOpenClawEnv(
  env: OpenClawEnvConfig,
): Promise<OpenClawWriteOutcome> {
  return requestWithBody<OpenClawWriteOutcome>(
    "/api/openclaw/env",
    "PUT",
    env,
  );
}

export async function getWebOpenClawTools(): Promise<OpenClawToolsConfig> {
  try {
    return await requestJson<OpenClawToolsConfig>("/api/openclaw/tools");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load openclaw tools config from local service",
      error,
    );
    return {};
  }
}

export async function setWebOpenClawTools(
  tools: OpenClawToolsConfig,
): Promise<OpenClawWriteOutcome> {
  return requestWithBody<OpenClawWriteOutcome>(
    "/api/openclaw/tools",
    "PUT",
    tools,
  );
}

export async function getWebOpenClawHealth(): Promise<OpenClawHealthWarning[]> {
  try {
    return await requestJson<OpenClawHealthWarning[]>("/api/openclaw/health");
  } catch (error) {
    console.warn(
      "[runtime:web] failed to load openclaw health warnings from local service",
      error,
    );
    return [];
  }
}

export async function getWebOpenClawLiveProvider(
  providerId: string,
): Promise<Record<string, unknown> | null> {
  try {
    return await requestJson<Record<string, unknown> | null>(
      `/api/openclaw/live-provider/${encodeURIComponent(providerId)}`,
    );
  } catch (error) {
    console.warn(
      `[runtime:web] failed to load openclaw live provider ${providerId} from local service`,
      error,
    );
    return null;
  }
}

export async function getWebSessions(): Promise<SessionMeta[]> {
  return requestJson<SessionMeta[]>("/api/sessions");
}

export async function getWebSessionMessages(
  providerId: string,
  sourcePath: string,
): Promise<SessionMessage[]> {
  return requestJson<SessionMessage[]>(
    `/api/sessions/messages?providerId=${encodeURIComponent(providerId)}&sourcePath=${encodeURIComponent(sourcePath)}`,
  );
}

export async function deleteWebSession(
  options: DeleteSessionOptions,
): Promise<boolean> {
  return requestWithBody<boolean>("/api/sessions", "DELETE", options);
}

export async function deleteWebSessions(
  items: DeleteSessionOptions[],
): Promise<DeleteSessionResult[]> {
  return requestWithBody<DeleteSessionResult[]>(
    "/api/sessions/delete-batch",
    "POST",
    items,
  );
}

export async function getWebUsageSummary(
  startDate?: number,
  endDate?: number,
): Promise<UsageSummary> {
  const params = new URLSearchParams();
  if (typeof startDate === "number") params.set("startDate", String(startDate));
  if (typeof endDate === "number") params.set("endDate", String(endDate));
  const query = params.toString();
  return requestJson<UsageSummary>(
    `/api/usage/summary${query ? `?${query}` : ""}`,
  );
}

export async function getWebUsageTrends(
  startDate?: number,
  endDate?: number,
): Promise<DailyStats[]> {
  const params = new URLSearchParams();
  if (typeof startDate === "number") params.set("startDate", String(startDate));
  if (typeof endDate === "number") params.set("endDate", String(endDate));
  const query = params.toString();
  return requestJson<DailyStats[]>(
    `/api/usage/trends${query ? `?${query}` : ""}`,
  );
}

export async function getWebProviderStats(): Promise<ProviderStats[]> {
  return requestJson<ProviderStats[]>("/api/usage/provider-stats");
}

export async function getWebModelStats(): Promise<ModelStats[]> {
  return requestJson<ModelStats[]>("/api/usage/model-stats");
}

export async function getWebRequestLogs(
  filters: LogFilters,
  page: number,
  pageSize: number,
): Promise<PaginatedLogs> {
  return requestWithBody<PaginatedLogs>("/api/usage/request-logs", "POST", {
    filters,
    page,
    pageSize,
  });
}

export async function getWebRequestDetail(
  requestId: string,
): Promise<RequestLog | null> {
  return requestJson<RequestLog | null>(
    `/api/usage/request-logs/${encodeURIComponent(requestId)}`,
  );
}

export async function getWebModelPricing(): Promise<ModelPricing[]> {
  return requestJson<ModelPricing[]>("/api/usage/model-pricing");
}

export async function updateWebModelPricing(
  modelId: string,
  displayName: string,
  inputCost: string,
  outputCost: string,
  cacheReadCost: string,
  cacheCreationCost: string,
): Promise<void> {
  return requestWithBody<void>(
    `/api/usage/model-pricing/${encodeURIComponent(modelId)}`,
    "PUT",
    {
      displayName,
      inputCost,
      outputCost,
      cacheReadCost,
      cacheCreationCost,
    },
  );
}

export async function deleteWebModelPricing(modelId: string): Promise<void> {
  return requestWithBody<void>(
    `/api/usage/model-pricing/${encodeURIComponent(modelId)}`,
    "DELETE",
  );
}

export async function getWebProviderLimits(
  providerId: string,
  appType: string,
): Promise<ProviderLimitStatus> {
  return requestJson<ProviderLimitStatus>(
    `/api/usage/provider-limits/${encodeURIComponent(appType)}/${encodeURIComponent(providerId)}`,
  );
}
