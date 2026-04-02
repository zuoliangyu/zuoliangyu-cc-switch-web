/**
 * GitHub Copilot API
 *
 * 提供 GitHub Copilot 可用模型、Token、使用量与账号管理相关的 API 函数。
 */

import { invoke } from "@/lib/runtime/client/core";

/**
 * GitHub 账号信息（公开信息）
 */
export interface GitHubAccount {
  /** GitHub 用户 ID（唯一标识） */
  id: string;
  /** GitHub 用户名 */
  login: string;
  /** 头像 URL */
  avatar_url: string | null;
  /** 认证时间戳（Unix 秒） */
  authenticated_at: number;
}

/**
 * 注销 Copilot 认证
 */
export async function copilotLogout(): Promise<void> {
  return invoke("copilot_logout");
}

/**
 * Copilot 可用模型
 */
export interface CopilotModel {
  id: string;
  name: string;
  vendor: string;
  model_picker_enabled: boolean;
}

/**
 * 获取有效的 Copilot Token
 *
 * 内部使用，用于代理请求。
 *
 * @returns Copilot Token
 */
export async function copilotGetToken(): Promise<string> {
  return invoke<string>("copilot_get_token");
}

/**
 * 获取 Copilot 可用模型列表
 *
 * @returns 可用模型列表
 */
export async function copilotGetModels(): Promise<CopilotModel[]> {
  return invoke<CopilotModel[]>("copilot_get_models");
}

/**
 * 配额详情
 */
export interface QuotaDetail {
  entitlement: number;
  remaining: number;
  percent_remaining: number;
  unlimited: boolean;
}

/**
 * 配额快照
 */
export interface QuotaSnapshots {
  chat: QuotaDetail;
  completions: QuotaDetail;
  premium_interactions: QuotaDetail;
}

/**
 * Copilot 使用量响应
 */
export interface CopilotUsageResponse {
  copilot_plan: string;
  quota_reset_date: string;
  quota_snapshots: QuotaSnapshots;
}

/**
 * 获取 Copilot 使用量信息
 *
 * @returns 使用量信息，包含计划类型、重置日期和配额快照
 */
export async function copilotGetUsage(): Promise<CopilotUsageResponse> {
  return invoke<CopilotUsageResponse>("copilot_get_usage");
}

// ==================== 多账号管理 API ====================

/**
 * 列出所有已认证的 GitHub 账号
 *
 * @returns 账号列表
 */
export async function copilotListAccounts(): Promise<GitHubAccount[]> {
  return invoke<GitHubAccount[]>("copilot_list_accounts");
}

/**
 * 轮询 OAuth Token（多账号版本）
 *
 * 使用设备码轮询 GitHub，等待用户完成授权。
 * 授权成功后返回新添加的账号信息。
 *
 * @param deviceCode - 设备码
 * @returns 新添加的账号信息，如果仍在等待则返回 null
 */
export async function copilotPollForAccount(
  deviceCode: string,
): Promise<GitHubAccount | null> {
  return invoke<GitHubAccount | null>("copilot_poll_for_account", {
    deviceCode,
  });
}

/**
 * 移除指定的 GitHub 账号
 *
 * @param accountId - GitHub 用户 ID
 */
export async function copilotRemoveAccount(accountId: string): Promise<void> {
  return invoke("copilot_remove_account", { accountId });
}

/**
 * 设置默认 GitHub 账号
 *
 * @param accountId - GitHub 用户 ID
 */
export async function copilotSetDefaultAccount(
  accountId: string,
): Promise<void> {
  return invoke("copilot_set_default_account", { accountId });
}

/**
 * 获取指定账号的有效 Copilot Token
 *
 * 内部使用，用于代理请求。
 *
 * @param accountId - GitHub 用户 ID
 * @returns Copilot Token
 */
export async function copilotGetTokenForAccount(
  accountId: string,
): Promise<string> {
  return invoke<string>("copilot_get_token_for_account", { accountId });
}

/**
 * 获取指定账号的 Copilot 可用模型列表
 *
 * @param accountId - GitHub 用户 ID
 * @returns 可用模型列表
 */
export async function copilotGetModelsForAccount(
  accountId: string,
): Promise<CopilotModel[]> {
  return invoke<CopilotModel[]>("copilot_get_models_for_account", {
    accountId,
  });
}

/**
 * 获取指定账号的 Copilot 使用量信息
 *
 * @param accountId - GitHub 用户 ID
 * @returns 使用量信息
 */
export async function copilotGetUsageForAccount(
  accountId: string,
): Promise<CopilotUsageResponse> {
  return invoke<CopilotUsageResponse>("copilot_get_usage_for_account", {
    accountId,
  });
}

