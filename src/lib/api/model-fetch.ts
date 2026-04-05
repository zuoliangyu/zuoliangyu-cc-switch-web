import { invoke } from "@/lib/runtime/client/core";
import type { TFunction } from "i18next";
import { toast } from "sonner";

export interface FetchedModel {
  id: string;
  ownedBy: string | null;
}

export async function fetchModelsForConfig(
  baseUrl: string,
  apiKey: string,
  isFullUrl?: boolean,
): Promise<FetchedModel[]> {
  return invoke<FetchedModel[]>("fetch_models_for_config", {
    baseUrl,
    apiKey,
    isFullUrl,
  });
}

export function showFetchModelsError(
  err: unknown,
  t: TFunction,
  opts?: { hasApiKey: boolean; hasBaseUrl: boolean },
): void {
  if (opts && !opts.hasBaseUrl && !opts.hasApiKey) {
    toast.error(t("providerForm.fetchModelsNeedConfig"));
    return;
  }
  if (opts && !opts.hasApiKey) {
    toast.error(t("providerForm.fetchModelsNeedApiKey"));
    return;
  }
  if (opts && !opts.hasBaseUrl) {
    toast.error(t("providerForm.fetchModelsNeedEndpoint"));
    return;
  }

  const message = String(err);

  if (message.includes("HTTP 401") || message.includes("HTTP 403")) {
    toast.error(t("providerForm.fetchModelsAuthFailed"));
    return;
  }
  if (message.includes("HTTP 404") || message.includes("HTTP 405")) {
    toast.error(t("providerForm.fetchModelsNotSupported"));
    return;
  }
  if (message.includes("timeout") || message.includes("timed out")) {
    toast.error(t("providerForm.fetchModelsTimeout"));
    return;
  }
  if (message.includes("Failed to parse")) {
    toast.error(t("providerForm.fetchModelsNotSupported"));
    return;
  }

  toast.error(t("providerForm.fetchModelsFailed"));
}
