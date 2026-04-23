import { invoke } from "@/lib/runtime/client/core";
import type { HermesMemoryKind, HermesMemoryLimits } from "@/types";

export const hermesApi = {
  async getMemory(kind: HermesMemoryKind): Promise<string> {
    return await invoke("get_hermes_memory", { kind });
  },

  async setMemory(kind: HermesMemoryKind, content: string): Promise<void> {
    await invoke("set_hermes_memory", { kind, content });
  },

  async getMemoryLimits(): Promise<HermesMemoryLimits> {
    return await invoke("get_hermes_memory_limits");
  },

  async setMemoryEnabled(
    kind: HermesMemoryKind,
    enabled: boolean,
  ): Promise<void> {
    await invoke("set_hermes_memory_enabled", { kind, enabled });
  },
};
