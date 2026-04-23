import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import MarkdownEditor from "@/components/MarkdownEditor";
import {
  useHermesMemory,
  useHermesMemoryLimits,
  useSaveHermesMemory,
  useToggleHermesMemoryEnabled,
} from "@/hooks/useHermes";
import type { HermesMemoryKind } from "@/types";
import { cn } from "@/lib/utils";

interface MemoryTabPaneProps {
  kind: HermesMemoryKind;
  limit: number;
  enabled: boolean;
}

function MemoryTabPane({ kind, limit, enabled }: MemoryTabPaneProps) {
  const { t } = useTranslation();
  const { data, isLoading } = useHermesMemory(kind, true);
  const saveMutation = useSaveHermesMemory();
  const toggleMutation = useToggleHermesMemoryEnabled();
  const [content, setContent] = useState("");
  const [loaded, setLoaded] = useState(false);
  const [isDarkMode, setIsDarkMode] = useState(false);

  useEffect(() => {
    if (!loaded && data !== undefined) {
      setContent(data);
      setLoaded(true);
    }
  }, [data, loaded]);

  useEffect(() => {
    setIsDarkMode(document.documentElement.classList.contains("dark"));

    const observer = new MutationObserver(() => {
      setIsDarkMode(document.documentElement.classList.contains("dark"));
    });

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });

    return () => observer.disconnect();
  }, []);

  const handleSave = async () => {
    try {
      await saveMutation.mutateAsync({ kind, content });
      toast.success(t("hermes.memory.saveSuccess"));
    } catch {
      // Toast is handled by the mutation hook.
    }
  };

  const charCount = content.length;
  const isOverLimit = charCount > limit;
  const filename = kind === "memory" ? "MEMORY.md" : "USER.md";

  return (
    <div className="flex flex-col gap-3">
      <div
        className={cn(
          "flex items-center justify-between rounded-md border px-3 py-2",
          enabled ? "bg-muted/30" : "border-amber-500/30 bg-amber-500/10",
        )}
      >
        <div className="flex items-center gap-2">
          <Switch
            checked={enabled}
            disabled={toggleMutation.isPending}
            onCheckedChange={(next) =>
              toggleMutation.mutate({ kind, enabled: next })
            }
          />
          <span className="text-sm">
            {enabled
              ? t("hermes.memory.enableOn")
              : t("hermes.memory.enableOff")}
          </span>
        </div>
        {!enabled && (
          <span className="text-xs text-amber-700 dark:text-amber-400">
            {t("hermes.memory.disabledHint")}
          </span>
        )}
      </div>

      {isLoading && !loaded ? (
        <div className="flex h-64 items-center justify-center text-muted-foreground">
          {t("common.loading")}
        </div>
      ) : (
        <MarkdownEditor
          value={content}
          onChange={setContent}
          darkMode={isDarkMode}
          placeholder={t("hermes.memory.placeholder", { filename })}
          minHeight="calc(100vh - 320px)"
        />
      )}

      <div className="flex items-center justify-between gap-3 text-sm">
        <span
          className={cn(
            "text-muted-foreground",
            isOverLimit && "font-medium text-red-600 dark:text-red-400",
          )}
        >
          {t("hermes.memory.usage", { current: charCount, limit })}
          {isOverLimit ? ` · ${t("hermes.memory.overLimit")}` : ""}
        </span>
        <div className="flex items-center gap-3">
          <span className="hidden text-xs text-muted-foreground md:inline">
            {t("hermes.memory.runtimeNote")}
          </span>
          <Button
            onClick={handleSave}
            disabled={saveMutation.isPending || !loaded}
          >
            {saveMutation.isPending ? t("common.saving") : t("common.save")}
          </Button>
        </div>
      </div>
    </div>
  );
}

const HermesMemoryPanel: React.FC = () => {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<HermesMemoryKind>("memory");
  const { data: limits } = useHermesMemoryLimits(true);

  return (
    <div className="flex h-full flex-col">
      <Tabs
        value={activeTab}
        onValueChange={(value) => setActiveTab(value as HermesMemoryKind)}
        className="flex flex-1 flex-col"
      >
        <div className="px-6 pt-4">
          <TabsList>
            <TabsTrigger value="memory">
              {t("hermes.memory.agentTab")}
            </TabsTrigger>
            <TabsTrigger value="user">{t("hermes.memory.userTab")}</TabsTrigger>
          </TabsList>
        </div>

        <TabsContent value="memory" className="mt-4 flex-1 px-6 pb-4">
          <MemoryTabPane
            kind="memory"
            limit={limits?.memory ?? 2200}
            enabled={limits?.memoryEnabled ?? true}
          />
        </TabsContent>
        <TabsContent value="user" className="mt-4 flex-1 px-6 pb-4">
          <MemoryTabPane
            kind="user"
            limit={limits?.user ?? 1375}
            enabled={limits?.userEnabled ?? true}
          />
        </TabsContent>
      </Tabs>
    </div>
  );
};

export default HermesMemoryPanel;
