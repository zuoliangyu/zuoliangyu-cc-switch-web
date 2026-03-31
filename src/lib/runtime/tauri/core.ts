import { getDefaultAppProxyConfig } from "./defaults";
import { isTauriRuntime } from "./env";
import {
  addWebProviderToFailoverQueue,
  addWebProvider,
  deleteWebProvider,
  getWebAutoFailoverEnabled,
  getWebAvailableProvidersForFailover,
  getWebCircuitBreakerConfig,
  getWebCircuitBreakerStats,
  getWebDefaultCostMultiplier,
  getWebFailoverQueue,
  getWebGlobalProxyConfig,
  getWebIsLiveTakeoverActive,
  getWebIsProxyRunning,
  getWebPricingModelSource,
  getWebMcpServers,
  getWebProviderHealth,
  getWebProxyConfig,
  getWebProxyConfigForApp,
  getWebProxyStatus,
  getWebProxyTakeoverStatus,
  getWebPrompts,
  getWebProviders,
  getWebSettings,
  getWebCurrentPromptFileContent,
  getWebInstalledSkills,
  getWebSkillBackups,
  getWebUnmanagedSkills,
  removeWebProviderFromFailoverQueue,
  saveWebSettings,
  setWebAutoFailoverEnabled,
  setWebDefaultCostMultiplier,
  setWebPricingModelSource,
  toggleWebMcpApp,
  setWebProxyTakeoverForApp,
  startWebProxyServer,
  stopWebProxyWithRestore,
  switchWebProvider,
  switchWebProxyProvider,
  importWebMcpFromApps,
  importWebPromptFromFile,
  importWebSkillsFromApps,
  upsertWebMcpServer,
  upsertWebPrompt,
  uninstallWebSkillUnified,
  updateWebCircuitBreakerConfig,
  updateWebGlobalProxyConfig,
  deleteWebMcpServer,
  deleteWebPrompt,
  toggleWebSkillApp,
  updateWebProvider,
  updateWebProxyConfig,
  updateWebProxyConfigForApp,
  enableWebPrompt,
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
    case "get_mcp_servers":
      return (await getWebMcpServers()) as T;
    case "upsert_mcp_server":
      return (await upsertWebMcpServer(args?.server as any)) as T;
    case "delete_mcp_server":
      return (await deleteWebMcpServer(args?.id as string)) as T;
    case "toggle_mcp_app":
      return (await toggleWebMcpApp(
        args?.serverId as string,
        args?.app as AppId,
        Boolean(args?.enabled),
      )) as T;
    case "import_mcp_from_apps":
      return (await importWebMcpFromApps()) as T;
    case "get_prompts":
      return (await getWebPrompts(args?.app as AppId)) as T;
    case "upsert_prompt":
      return (await upsertWebPrompt(
        args?.app as AppId,
        args?.id as string,
        args?.prompt as any,
      )) as T;
    case "delete_prompt":
      return (await deleteWebPrompt(
        args?.app as AppId,
        args?.id as string,
      )) as T;
    case "enable_prompt":
      return (await enableWebPrompt(
        args?.app as AppId,
        args?.id as string,
      )) as T;
    case "import_prompt_from_file":
      return (await importWebPromptFromFile(args?.app as AppId)) as T;
    case "get_current_prompt_file_content":
      return (await getWebCurrentPromptFileContent(args?.app as AppId)) as T;
    case "get_installed_skills":
      return (await getWebInstalledSkills()) as T;
    case "get_skill_backups":
      return (await getWebSkillBackups()) as T;
    case "scan_unmanaged_skills":
      return (await getWebUnmanagedSkills()) as T;
    case "import_skills_from_apps":
      return (await importWebSkillsFromApps(args?.imports as any[])) as T;
    case "uninstall_skill_unified":
      return (await uninstallWebSkillUnified(args?.id as string)) as T;
    case "toggle_skill_app":
      return (await toggleWebSkillApp(
        args?.id as string,
        args?.app as AppId,
        Boolean(args?.enabled),
      )) as T;
    case "start_proxy_server":
      return (await startWebProxyServer()) as T;
    case "stop_proxy_with_restore":
      return (await stopWebProxyWithRestore()) as T;
    case "get_proxy_status":
      return (await getWebProxyStatus()) as T;
    case "get_proxy_takeover_status":
      return (await getWebProxyTakeoverStatus()) as T;
    case "set_proxy_takeover_for_app":
      return (await setWebProxyTakeoverForApp(
        args?.appType as AppId,
        Boolean(args?.enabled),
      )) as T;
    case "get_proxy_config":
      return (await getWebProxyConfig()) as T;
    case "update_proxy_config":
      return (await updateWebProxyConfig(args?.config as any)) as T;
    case "get_global_proxy_config":
      return (await getWebGlobalProxyConfig()) as T;
    case "update_global_proxy_config":
      return (await updateWebGlobalProxyConfig(args?.config as any)) as T;
    case "get_proxy_config_for_app": {
      const appType = args?.appType as AppId | undefined;
      return (
        appType
          ? await getWebProxyConfigForApp(appType)
          : getDefaultAppProxyConfig()
      ) as T;
    }
    case "update_proxy_config_for_app":
      return (await updateWebProxyConfigForApp(args?.config as any)) as T;
    case "get_default_cost_multiplier":
      return (await getWebDefaultCostMultiplier(args?.appType as AppId)) as T;
    case "set_default_cost_multiplier":
      return (await setWebDefaultCostMultiplier(
        args?.appType as AppId,
        args?.value as string,
      )) as T;
    case "get_pricing_model_source":
      return (await getWebPricingModelSource(args?.appType as AppId)) as T;
    case "set_pricing_model_source":
      return (await setWebPricingModelSource(
        args?.appType as AppId,
        args?.value as string,
      )) as T;
    case "is_proxy_running":
      return (await getWebIsProxyRunning()) as T;
    case "is_live_takeover_active":
      return (await getWebIsLiveTakeoverActive()) as T;
    case "switch_proxy_provider":
      return (await switchWebProxyProvider(
        args?.appType as AppId,
        args?.providerId as string,
      )) as T;
    case "get_provider_health":
      return (await getWebProviderHealth(
        args?.appType as AppId,
        args?.providerId as string,
      )) as T;
    case "get_circuit_breaker_config":
      return (await getWebCircuitBreakerConfig()) as T;
    case "update_circuit_breaker_config":
      return (await updateWebCircuitBreakerConfig(args?.config as any)) as T;
    case "get_circuit_breaker_stats":
      return (await getWebCircuitBreakerStats(
        args?.appType as AppId,
        args?.providerId as string,
      )) as T;
    case "get_failover_queue":
      return (await getWebFailoverQueue(args?.appType as AppId)) as T;
    case "get_available_providers_for_failover":
      return (await getWebAvailableProvidersForFailover(
        args?.appType as AppId,
      )) as T;
    case "add_to_failover_queue":
      return (await addWebProviderToFailoverQueue(
        args?.appType as AppId,
        args?.providerId as string,
      )) as T;
    case "remove_from_failover_queue":
      return (await removeWebProviderFromFailoverQueue(
        args?.appType as AppId,
        args?.providerId as string,
      )) as T;
    case "get_auto_failover_enabled":
      return (await getWebAutoFailoverEnabled(args?.appType as AppId)) as T;
    case "set_auto_failover_enabled":
      return (await setWebAutoFailoverEnabled(
        args?.appType as AppId,
        Boolean(args?.enabled),
      )) as T;
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
