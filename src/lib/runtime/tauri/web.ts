import type { Settings } from "@/types";
import type { Provider } from "@/types";
import type { AppId } from "@/lib/api";
import type { SwitchResult } from "@/lib/api/providers";
import type {
  AppProxyConfig,
  GlobalProxyConfig,
  ProxyConfig,
  ProxyServerInfo,
  ProxyStatus,
  ProxyTakeoverStatus,
} from "@/types/proxy";
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

async function requestJson<T>(path: string): Promise<T> {
  const response = await fetch(`${getWebApiBase()}${path}`, {
    headers: {
      Accept: "application/json",
    },
  });

  if (!response.ok) {
    throw new Error(`HTTP ${response.status} for ${path}`);
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
    throw new Error(`HTTP ${response.status} for ${method} ${path}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

export async function getWebSettings(): Promise<Settings> {
  try {
    return await requestJson<Settings>("/api/settings");
  } catch (error) {
    console.warn("[runtime:web] failed to load settings from local service", error);
    return getDefaultSettings();
  }
}

export async function getWebProviders(appId: AppId): Promise<ProvidersResponse> {
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

export async function getWebProxyStatus(): Promise<ProxyStatus> {
  try {
    return await requestJson<ProxyStatus>("/api/proxy/status");
  } catch (error) {
    console.warn("[runtime:web] failed to load proxy status from local service", error);
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
    console.warn("[runtime:web] failed to load proxy config from local service", error);
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

export async function saveWebSettings(settings: Settings): Promise<boolean> {
  return requestWithBody<boolean>("/api/settings", "PUT", settings);
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
