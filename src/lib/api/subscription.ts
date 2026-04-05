import { invoke } from "@/lib/runtime/client/core";
import type { SubscriptionQuota } from "@/types/subscription";

export const subscriptionApi = {
  getQuota(tool: string): Promise<SubscriptionQuota> {
    return invoke("get_subscription_quota", { tool });
  },
};
