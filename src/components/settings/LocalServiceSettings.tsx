import { Rocket } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ToggleRow } from "@/components/ui/toggle-row";

interface LocalServiceSettingsProps {
  launchOnStartup: boolean;
  onLaunchOnStartupChange: (value: boolean) => void;
}

export function LocalServiceSettings({
  launchOnStartup,
  onLaunchOnStartupChange,
}: LocalServiceSettingsProps) {
  const { t } = useTranslation();

  return (
    <section className="space-y-4">
      <div className="flex items-center gap-2 border-b border-border/40 pb-2">
        <Rocket className="h-4 w-4 text-primary" />
        <div className="space-y-1">
          <h3 className="text-sm font-medium">{t("settings.localService")}</h3>
          <p className="text-xs text-muted-foreground">
            {t("settings.localServiceHint")}
          </p>
        </div>
      </div>

      <ToggleRow
        icon={<Rocket className="h-4 w-4 text-orange-500" />}
        title={t("settings.launchOnStartup")}
        description={t("settings.launchOnStartupDescription")}
        checked={launchOnStartup}
        onCheckedChange={onLaunchOnStartupChange}
      />
    </section>
  );
}
