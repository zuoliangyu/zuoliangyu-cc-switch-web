import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, Download, Link2, Wand2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { ProviderIcon } from "@/components/ProviderIcon";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { extractErrorMessage } from "@/utils/errorUtils";
import { decodeBase64Utf8 } from "@/lib/utils/base64";
import {
  extractDeepLinkFromLocation,
  mergeDeepLinkConfig,
  parseDeepLinkConfigObject,
  parseDeepLinkUrl,
} from "@/lib/deeplink/parser";
import { importFromDeepLink } from "@/lib/deeplink/importer";
import type { DeepLinkImportRequest } from "@/lib/deeplink/types";

const OPEN_EVENT = "cc-switch-open-deeplink-import";

const maskValue = (key: string, value: string) => {
  const sensitive = ["TOKEN", "KEY", "SECRET", "PASSWORD"].some((token) =>
    key.toUpperCase().includes(token),
  );
  if (!sensitive || value.length <= 8) {
    return value;
  }
  return `${value.slice(0, 8)}************`;
};

const getTitleKey = (request: DeepLinkImportRequest | null) => {
  switch (request?.resource) {
    case "prompt":
      return "deeplink.importPrompt";
    case "mcp":
      return "deeplink.importMcp";
    case "skill":
      return "deeplink.importSkill";
    default:
      return "deeplink.confirmImport";
  }
};

const getDescriptionKey = (request: DeepLinkImportRequest | null) => {
  switch (request?.resource) {
    case "prompt":
      return "deeplink.importPromptDescription";
    case "mcp":
      return "deeplink.importMcpDescription";
    case "skill":
      return "deeplink.importSkillDescription";
    default:
      return "deeplink.confirmImportDescription";
  }
};

export function DeepLinkImportDialog() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [isParsing, setIsParsing] = useState(false);
  const [rawValue, setRawValue] = useState("");
  const [request, setRequest] = useState<DeepLinkImportRequest | null>(null);
  const [parseError, setParseError] = useState<string | null>(null);

  const resetState = () => {
    setRawValue("");
    setRequest(null);
    setParseError(null);
    setIsImporting(false);
    setIsParsing(false);
  };

  const parseAndOpen = async (value: string) => {
    const trimmed = value.trim();
    if (!trimmed) {
      setOpen(true);
      return;
    }

    setIsParsing(true);
    try {
      const parsed = parseDeepLinkUrl(trimmed);
      const merged = await mergeDeepLinkConfig(parsed);
      setRawValue(trimmed);
      setRequest(merged);
      setParseError(null);
      setOpen(true);
    } catch (error) {
      setRawValue(trimmed);
      setRequest(null);
      setParseError(extractErrorMessage(error));
      setOpen(true);
    } finally {
      setIsParsing(false);
    }
  };

  useEffect(() => {
    const autoDeepLink = extractDeepLinkFromLocation(window.location);
    if (!autoDeepLink) {
      return;
    }

    void parseAndOpen(autoDeepLink);

    const nextUrl = new URL(window.location.href);
    nextUrl.searchParams.delete("deeplink");
    window.history.replaceState(
      {},
      "",
      `${nextUrl.pathname}${nextUrl.search}${nextUrl.hash}`,
    );
  }, []);

  useEffect(() => {
    const handler = (event: Event) => {
      const detail = (event as CustomEvent<{ deeplink?: string }>).detail;
      void parseAndOpen(detail?.deeplink ?? "");
    };
    window.addEventListener(OPEN_EVENT, handler as EventListener);
    return () =>
      window.removeEventListener(OPEN_EVENT, handler as EventListener);
  }, []);

  const parsedConfig = useMemo(() => {
    if (!request) {
      return null;
    }

    try {
      return parseDeepLinkConfigObject(request);
    } catch {
      return null;
    }
  }, [request]);

  const decodedPromptContent = useMemo(() => {
    if (!request?.content) {
      return "";
    }
    try {
      return decodeBase64Utf8(request.content);
    } catch {
      return "";
    }
  }, [request?.content]);

  const mcpServerCount = useMemo(() => {
    if (request?.resource !== "mcp" || !parsedConfig) {
      return 0;
    }
    if (
      "mcpServers" in parsedConfig &&
      parsedConfig.mcpServers &&
      typeof parsedConfig.mcpServers === "object"
    ) {
      return Object.keys(
        parsedConfig.mcpServers as Record<string, unknown>,
      ).length;
    }
    if (typeof parsedConfig === "object") {
      return Object.keys(parsedConfig).length;
    }
    return 0;
  }, [parsedConfig, request?.resource]);

  const handleParse = () => {
    void parseAndOpen(rawValue);
  };

  const handleImport = async () => {
    if (!request) {
      return;
    }

    setIsImporting(true);
    try {
      const result = await importFromDeepLink(request);

      if (result.type === "provider") {
        await queryClient.invalidateQueries({
          queryKey: ["providers"],
          refetchType: "all",
        });
        toast.success(t("deeplink.importSuccess"), {
          description: t("deeplink.importSuccessDescription", {
            name: request.name,
          }),
          closeButton: true,
        });
      } else if (result.type === "prompt") {
        window.dispatchEvent(
          new CustomEvent("prompt-imported", {
            detail: { app: request.app, id: result.id },
          }),
        );
        toast.success(t("deeplink.promptImportSuccess"), {
          description: t("deeplink.promptImportSuccessDescription", {
            name: request.name,
          }),
          closeButton: true,
        });
      } else if (result.type === "mcp") {
        await queryClient.invalidateQueries({
          queryKey: ["mcp", "all"],
          refetchType: "all",
        });
        if (result.failed.length > 0) {
          toast.warning(t("deeplink.mcpPartialSuccess"), {
            description: t("deeplink.mcpPartialSuccessDescription", {
              success: result.importedCount,
              failed: result.failed.length,
            }),
            closeButton: true,
          });
        } else {
          toast.success(t("deeplink.mcpImportSuccess"), {
            description: t("deeplink.mcpImportSuccessDescription", {
              count: result.importedCount,
            }),
            closeButton: true,
          });
        }
      } else if (result.type === "skill") {
        await queryClient.invalidateQueries({
          queryKey: ["skills"],
          refetchType: "all",
        });
        toast.success(t("deeplink.skillImportSuccess"), {
          description: t("deeplink.skillImportSuccessDescription", {
            repo: result.key,
          }),
          closeButton: true,
        });
      }

      setOpen(false);
      resetState();
    } catch (error) {
      toast.error(t("deeplink.importError"), {
        description: extractErrorMessage(error),
      });
    } finally {
      setIsImporting(false);
    }
  };

  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen);
    if (!nextOpen) {
      resetState();
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader className="text-left">
          <DialogTitle>{t(getTitleKey(request))}</DialogTitle>
          <DialogDescription>{t(getDescriptionKey(request))}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-2">
            <label className="text-sm font-medium text-foreground">
              {t("deeplink.manualInput")}
            </label>
            <Textarea
              value={rawValue}
              onChange={(event) => setRawValue(event.target.value)}
              rows={4}
              placeholder="ccswitch://v1/import?resource=provider&app=claude&name=..."
              className="font-mono text-xs"
            />
            <div className="flex flex-wrap items-center gap-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={handleParse}
                disabled={isParsing}
              >
                <Wand2 className="mr-2 h-4 w-4" />
                {t("deeplink.parse")}
              </Button>
              <p className="text-xs text-muted-foreground">
                {t("deeplink.manualInputHint")}
              </p>
            </div>
          </div>

          {parseError && (
            <div className="rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
              {parseError}
            </div>
          )}

          {request && (
            <div className="max-h-[52vh] space-y-4 overflow-y-auto rounded-xl border border-border bg-muted/30 p-4">
              {request.resource === "provider" && (
                <div className="space-y-3">
                  {request.icon && (
                    <div className="flex justify-center pb-1 pt-2">
                      <ProviderIcon
                        icon={request.icon}
                        name={request.name || request.icon}
                        size={80}
                        className="drop-shadow-sm"
                      />
                    </div>
                  )}
                  <InfoRow
                    label={t("deeplink.app")}
                    value={request.app ?? "-"}
                  />
                  <InfoRow
                    label={t("deeplink.providerName")}
                    value={request.name ?? "-"}
                  />
                  <InfoRow
                    label={t("deeplink.homepage")}
                    value={request.homepage ?? "-"}
                    mono
                  />
                  <div className="grid gap-1 sm:grid-cols-[120px_1fr] sm:gap-3">
                    <div className="text-sm font-medium text-muted-foreground">
                      {t("deeplink.endpoint")}
                    </div>
                    <div className="space-y-1 text-sm font-mono break-all">
                      {(request.endpoint ?? "-")
                        .split(",")
                        .map((endpoint) => endpoint.trim())
                        .filter(Boolean)
                        .map((endpoint, index, items) => (
                          <div
                            key={`${endpoint}-${index}`}
                            className={
                              index === 0 ? "font-medium" : "text-muted-foreground"
                            }
                          >
                            {index === 0 ? "🔹 " : "└ "}
                            {endpoint}
                            {index === 0 && items.length > 1 && (
                              <span className="ml-2 text-xs text-muted-foreground">
                                ({t("deeplink.primaryEndpoint")})
                              </span>
                            )}
                          </div>
                        ))}
                    </div>
                  </div>
                  <InfoRow
                    label={t("deeplink.apiKey")}
                    value={
                      request.apiKey
                        ? maskValue("apiKey", request.apiKey)
                        : "****"
                    }
                    mono
                  />
                  {request.model && (
                    <InfoRow
                      label={t("deeplink.model")}
                      value={request.model}
                      mono
                    />
                  )}
                  {request.haikuModel && (
                    <InfoRow
                      label={t("deeplink.haikuModel")}
                      value={request.haikuModel}
                      mono
                    />
                  )}
                  {request.sonnetModel && (
                    <InfoRow
                      label={t("deeplink.sonnetModel")}
                      value={request.sonnetModel}
                      mono
                    />
                  )}
                  {request.opusModel && (
                    <InfoRow
                      label={t("deeplink.opusModel")}
                      value={request.opusModel}
                      mono
                    />
                  )}
                  {request.notes && (
                    <InfoRow
                      label={t("deeplink.notes")}
                      value={request.notes}
                    />
                  )}
                  {(request.config || request.configUrl) && (
                    <InfoRow
                      label={t("deeplink.configSource")}
                      value={
                        request.configUrl
                          ? t("deeplink.configRemote")
                          : t("deeplink.configEmbedded")
                      }
                    />
                  )}
                  {request.configUrl && (
                    <InfoRow
                      label={t("deeplink.configUrl")}
                      value={request.configUrl}
                      mono
                    />
                  )}
                  {parsedConfig && (
                    <PreviewCard title={t("deeplink.configDetails")}>
                      <pre className="overflow-x-auto whitespace-pre-wrap text-xs">
                        {JSON.stringify(parsedConfig, null, 2).slice(0, 1200)}
                      </pre>
                    </PreviewCard>
                  )}
                  {(request.usageScript ||
                    request.usageEnabled !== undefined ||
                    request.usageApiKey ||
                    request.usageBaseUrl ||
                    request.usageAutoInterval !== undefined) && (
                    <PreviewCard title={t("deeplink.usageScript")}>
                      <div className="space-y-3">
                        <InfoRow
                          label={t("deeplink.usageScript")}
                          value={
                            request.usageEnabled !== false
                              ? t("deeplink.usageScriptEnabled")
                              : t("deeplink.usageScriptDisabled")
                          }
                        />
                        {request.usageApiKey &&
                          request.usageApiKey !== request.apiKey && (
                            <InfoRow
                              label={t("deeplink.usageApiKey")}
                              value={maskValue(
                                "usageApiKey",
                                request.usageApiKey,
                              )}
                              mono
                            />
                          )}
                        {request.usageBaseUrl &&
                          request.usageBaseUrl !== request.endpoint && (
                            <InfoRow
                              label={t("deeplink.usageBaseUrl")}
                              value={request.usageBaseUrl}
                              mono
                            />
                          )}
                        {request.usageAutoInterval &&
                          request.usageAutoInterval > 0 && (
                            <InfoRow
                              label={t("deeplink.usageAutoInterval")}
                              value={t("deeplink.usageAutoIntervalValue", {
                                minutes: request.usageAutoInterval,
                              })}
                            />
                          )}
                      </div>
                    </PreviewCard>
                  )}
                </div>
              )}

              {request.resource === "prompt" && (
                <div className="space-y-3">
                  <InfoRow
                    label={t("deeplink.prompt.app")}
                    value={request.app ?? "-"}
                  />
                  <InfoRow
                    label={t("deeplink.prompt.name")}
                    value={request.name ?? "-"}
                  />
                  {request.description && (
                    <InfoRow
                      label={t("deeplink.prompt.description")}
                      value={request.description}
                    />
                  )}
                  <PreviewCard title={t("deeplink.prompt.contentPreview")}>
                    <pre className="overflow-x-auto whitespace-pre-wrap text-xs">
                      {decodedPromptContent.slice(0, 1200)}
                      {decodedPromptContent.length > 1200 ? "..." : ""}
                    </pre>
                  </PreviewCard>
                  {request.enabled && (
                    <div className="rounded-lg border border-yellow-500/30 bg-yellow-500/10 p-3 text-sm text-yellow-700 dark:text-yellow-300">
                      {t("deeplink.prompt.enabledWarning")}
                    </div>
                  )}
                </div>
              )}

              {request.resource === "mcp" && (
                <div className="space-y-3">
                  <InfoRow
                    label={t("deeplink.mcp.targetApps")}
                    value={request.apps ?? "-"}
                  />
                  <div className="text-sm text-muted-foreground">
                    {t("deeplink.mcp.serverCount", { count: mcpServerCount })}
                  </div>
                  <PreviewCard title={t("deeplink.mcp.serverPreview")}>
                    <pre className="overflow-x-auto whitespace-pre-wrap text-xs">
                      {JSON.stringify(
                        (parsedConfig?.mcpServers as
                          | Record<string, unknown>
                          | undefined) ??
                          parsedConfig ??
                          {},
                        null,
                        2,
                      ).slice(0, 1600)}
                    </pre>
                  </PreviewCard>
                  {request.enabled && (
                    <div className="rounded-lg border border-yellow-500/30 bg-yellow-500/10 p-3 text-sm text-yellow-700 dark:text-yellow-300">
                      {t("deeplink.mcp.enabledWarning")}
                    </div>
                  )}
                </div>
              )}

              {request.resource === "skill" && (
                <div className="space-y-3">
                  <InfoRow
                    label={t("deeplink.skill.repo")}
                    value={request.repo ?? "-"}
                    mono
                  />
                  <InfoRow
                    label={t("deeplink.skill.directory")}
                    value={request.directory ?? "-"}
                    mono
                  />
                  <InfoRow
                    label={t("deeplink.skill.branch")}
                    value={request.branch || "main"}
                    mono
                  />
                </div>
              )}

              <div className="rounded-lg border border-yellow-500/30 bg-yellow-500/10 p-3 text-sm text-yellow-700 dark:text-yellow-300">
                <div className="flex items-start gap-2">
                  <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
                  <span>{t("deeplink.warning")}</span>
                </div>
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => handleOpenChange(false)}
            disabled={isImporting || isParsing}
          >
            {t("common.cancel")}
          </Button>
          <Button
            onClick={handleImport}
            disabled={!request || isImporting || isParsing}
          >
            {isImporting ? (
              <>
                <Download className="mr-2 h-4 w-4 animate-pulse" />
                {t("deeplink.importing")}
              </>
            ) : (
              <>
                <Link2 className="mr-2 h-4 w-4" />
                {t("deeplink.import")}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function InfoRow({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="grid gap-1 sm:grid-cols-[120px_1fr] sm:gap-3">
      <div className="text-sm font-medium text-muted-foreground">{label}</div>
      <div
        className={mono ? "break-all font-mono text-sm" : "break-words text-sm"}
      >
        {value}
      </div>
    </div>
  );
}

function PreviewCard({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="space-y-2 rounded-lg border border-border bg-background/80 p-3">
      <div className="text-sm font-medium text-muted-foreground">{title}</div>
      {children}
    </div>
  );
}
