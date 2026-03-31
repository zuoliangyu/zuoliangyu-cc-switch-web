import {
  getDefaultAppProxyConfig,
} from "./defaults";
import { isTauriRuntime } from "./env";
import {
  addWebProvider,
  deleteWebProvider,
  getWebGlobalProxyConfig,
  getWebIsLiveTakeoverActive,
  getWebIsProxyRunning,
  getWebProxyConfig,
  getWebProxyConfigForApp,
  getWebProxyStatus,
  getWebProxyTakeoverStatus,
  getWebProviders,
  getWebSettings,
  saveWebSettings,
  switchWebProvider,
  updateWebProvider,
} from "./web";

type AppId = "claude" | "codex" | "gemini" | "opencode" | "openclaw";

type InvokeArgs = Record<string, unknown> | undefined;

const webUnsupportedError = (command: string): Error =>
  new Error(`[runtime:web] Tauri command not available yet: ${command}`);

export async function invoke<T>(
  command: string,
  args?: InvokeArgs,
): Promise<T> {
  if (isTauriRuntime()) {
    const mod = await import("@tauri-apps/api/core");
    return mod.invoke<T>(command, args);
  }

  switch (command) {
    case "get_init_error":
      return null as T;
    case "get_migration_result":
      return false as T;
    case "get_skills_migration_result":
      return null as T;
    case "get_settings":
      return (await getWebSettings()) as T;
    case "save_settings":
      return (await saveWebSettings(args?.settings as any)) as T;
    case "get_providers": {
      const appId = args?.app as AppId | undefined;
      if (!appId) {
        return {} as T;
      }
      const result = await getWebProviders(appId);
      return result.providers as T;
    }
    case "get_current_provider": {
      const appId = args?.app as AppId | undefined;
      if (!appId) {
        return "" as T;
      }
      const result = await getWebProviders(appId);
      return result.currentProviderId as T;
    }
    case "add_provider":
      return (await addWebProvider(
        args?.app as AppId,
        args?.provider as any,
      )) as T;
    case "update_provider":
      return (await updateWebProvider(
        args?.app as AppId,
        args?.provider as any,
      )) as T;
    case "delete_provider":
      return (await deleteWebProvider(
        args?.app as AppId,
        args?.id as string,
      )) as T;
    case "switch_provider":
      return (await switchWebProvider(
        args?.app as AppId,
        args?.id as string,
      )) as T;
    case "get_universal_providers":
      return {} as T;
    case "get_universal_provider":
      return null as T;
    case "get_opencode_live_provider_ids":
    case "get_openclaw_live_provider_ids":
    case "get_tool_versions":
      return [] as T;
    case "get_proxy_status":
      return (await getWebProxyStatus()) as T;
    case "get_proxy_takeover_status":
      return (await getWebProxyTakeoverStatus()) as T;
    case "get_proxy_config":
      return (await getWebProxyConfig()) as T;
    case "get_global_proxy_config":
      return (await getWebGlobalProxyConfig()) as T;
    case "get_proxy_config_for_app": {
      const appType = args?.appType as AppId | undefined;
      return (appType
        ? await getWebProxyConfigForApp(appType)
        : getDefaultAppProxyConfig()) as T;
    }
    case "is_proxy_running":
      return (await getWebIsProxyRunning()) as T;
    case "is_live_takeover_active":
      return (await getWebIsLiveTakeoverActive()) as T;
    case "get_app_config_dir_override":
      return null as T;
    case "get_config_dir":
      return "" as T;
    case "update_tray_menu":
      return false as T;
    case "set_window_theme":
      return true as T;
    case "open_external": {
      const url = typeof args?.url === "string" ? args.url : undefined;
      if (typeof window !== "undefined" && url) {
        window.open(url, "_blank", "noopener,noreferrer");
      }
      return undefined as T;
    }
    default:
      throw webUnsupportedError(command);
  }
}

