import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { hermesApi } from "@/lib/api/hermes";
import type { HermesMemoryKind } from "@/types";
import { extractErrorMessage } from "@/utils/errorUtils";

export const hermesKeys = {
  all: ["hermes"] as const,
  memory: (kind: HermesMemoryKind) => ["hermes", "memory", kind] as const,
  memoryLimits: ["hermes", "memoryLimits"] as const,
};

export function useHermesMemory(kind: HermesMemoryKind, enabled: boolean) {
  return useQuery({
    queryKey: hermesKeys.memory(kind),
    queryFn: () => hermesApi.getMemory(kind),
    enabled,
  });
}

export function useHermesMemoryLimits(enabled: boolean) {
  return useQuery({
    queryKey: hermesKeys.memoryLimits,
    queryFn: () => hermesApi.getMemoryLimits(),
    staleTime: 60_000,
    enabled,
  });
}

export function useSaveHermesMemory() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: ({
      kind,
      content,
    }: {
      kind: HermesMemoryKind;
      content: string;
    }) => hermesApi.setMemory(kind, content),
    onSuccess: async (_data, variables) => {
      await queryClient.invalidateQueries({
        queryKey: hermesKeys.memory(variables.kind),
      });
    },
    onError: (error) => {
      toast.error(t("hermes.memory.saveFailed"), {
        description: extractErrorMessage(error) || undefined,
      });
    },
  });
}

export function useToggleHermesMemoryEnabled() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: ({
      kind,
      enabled,
    }: {
      kind: HermesMemoryKind;
      enabled: boolean;
    }) => hermesApi.setMemoryEnabled(kind, enabled),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: hermesKeys.memoryLimits,
      });
    },
    onError: (error) => {
      toast.error(t("hermes.memory.toggleFailed"), {
        description: extractErrorMessage(error) || undefined,
      });
    },
  });
}
