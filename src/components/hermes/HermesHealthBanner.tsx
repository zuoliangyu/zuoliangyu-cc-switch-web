import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { TriangleAlert } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import type { HermesHealthWarning } from "@/types";

interface HermesHealthBannerProps {
  warnings: HermesHealthWarning[];
}

function getWarningText(
  code: string,
  fallback: string,
  t: ReturnType<typeof useTranslation>["t"],
) {
  switch (code) {
    case "config_parse_failed":
      return t("hermes.health.parseFailed");
    case "model_no_default":
      return t("hermes.health.modelNoDefault");
    case "custom_providers_not_list":
      return t("hermes.health.customProvidersNotList");
    case "model_provider_unknown":
      return t("hermes.health.modelProviderUnknown");
    case "model_default_not_in_provider":
      return t("hermes.health.modelDefaultNotInProvider");
    case "duplicate_provider_name":
      return t("hermes.health.duplicateProviderName");
    case "duplicate_provider_base_url":
      return t("hermes.health.duplicateProviderBaseUrl");
    case "schema_migrated_v12":
      return t("hermes.health.schemaMigratedV12");
    default:
      return fallback;
  }
}

const HermesHealthBanner: React.FC<HermesHealthBannerProps> = ({
  warnings,
}) => {
  const { t } = useTranslation();

  const items = useMemo(
    () =>
      warnings.map((warning) => ({
        ...warning,
        text: getWarningText(warning.code, warning.message, t),
      })),
    [t, warnings],
  );

  if (warnings.length === 0) {
    return null;
  }

  return (
    <div className="px-6 pt-4">
      <Alert className="border-amber-500/30 bg-amber-500/5">
        <TriangleAlert className="h-4 w-4" />
        <AlertTitle>
          {t("hermes.health.title", {
            defaultValue: "Hermes config warnings detected",
          })}
        </AlertTitle>
        <AlertDescription>
          <ul className="list-disc space-y-1 pl-5">
            {items.map((warning) => (
              <li key={`${warning.code}:${warning.path ?? warning.message}`}>
                {warning.text}
                {warning.path ? ` (${warning.path})` : ""}
              </li>
            ))}
          </ul>
        </AlertDescription>
      </Alert>
    </div>
  );
};

export default HermesHealthBanner;
