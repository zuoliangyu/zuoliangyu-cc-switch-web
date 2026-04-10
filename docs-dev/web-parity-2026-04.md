# Web 端对齐记录（2026-04）

本轮对齐目标：把桌面端已新增、而 Web 端缺失的关键能力补齐到当前 Web 分支。

本轮已落地：

- 供应商模型拉取
  - Claude / Codex / Gemini / OpenClaw 表单支持直接拉取模型列表
- 官方订阅额度展示
  - Claude / Codex / Gemini 卡片底部展示官方订阅额度信息
- 环境变量冲突检测
  - 启动与切换应用时检测冲突
  - 提供批量删除与备份恢复接口
- Deep Link 导入
  - 支持页面 URL 参数 `?deeplink=...`
  - 支持在设置页手动粘贴 `ccswitch://...` 导入
  - 覆盖 provider / prompt / mcp / skill repo 四类资源
- About 更新入口
  - Web 端不做自动下载与安装
  - 改为直接打开 GitHub Releases latest
- Codex OAuth 托管认证闭环
  - Web 后端新增 `codex_oauth` 多账号 Device Code 登录与 token 刷新管理
  - 认证中心新增 ChatGPT (Codex OAuth) 账号管理区块
  - Claude 供应商预设补齐 `Codex` OAuth 预设，并支持保存 `authBinding`
  - Claude 代理链路支持 `codex_oauth` 动态注入 access token 与 `ChatGPT-Account-Id`
  - Provider 卡片新增 Codex OAuth 账号绑定额度展示
- Skills 更新能力
  - `skills` 表补齐 `content_hash` / `updated_at` 字段，并新增 v7 schema 迁移
  - Web 后端新增 Skills 更新检查与单项更新接口
  - Skills 面板新增“检查更新 / 全部更新 / 单项更新”入口
  - 更新后会重算哈希、刷新 Skill 元数据，并同步回启用中的应用目录
- skills.sh 搜索
  - Web 后端新增 `skills.sh` 搜索接口，并过滤非 GitHub 来源条目
  - Skills 发现页新增仓库 / `skills.sh` 双搜索源切换
  - 支持分页加载更多，并复用现有安装链路直接安装搜索结果
- Skill 存储位置切换
  - Web 设置新增 Skill 存储位置切换入口，支持 `~/.cc-switch/skills` 与 `~/.agents/skills` 双目录
  - Web 后端新增 Skill 存储迁移接口，迁移完成后自动刷新各应用目录同步
  - 设置数据链路补齐 `skillStorageLocation` 字段，并同步三语文案
- 首次使用提示补齐
  - Web 设置持久化补齐 `firstRunNoticeConfirmed` / `commonConfigConfirmed` 字段
  - 主页面补上首次运行欢迎提示弹窗
  - 供应商表单补上“通用配置”首次说明确认弹窗，并同步三语文案
- 按供应商打开终端
  - Web 后端新增 `POST /api/providers/:app/:id/open-terminal`，支持按供应商配置启动专属 Claude 终端
  - Provider 列表补上“打开终端”动作按钮，当前在 Claude 供应商视图中可用
  - 保留可选 `cwd` 参数链路，当前 Web 版前端默认直接打开终端，不额外接目录选择器
  - 三语文案补齐 `provider.openTerminal` / `provider.terminalOpened` / `provider.terminalOpenFailed`
- 原生余额 / Token Plan 模板
  - Web 后端补齐第三方余额查询服务（DeepSeek / StepFun / SiliconFlow / OpenRouter / Novita AI）
  - Web 后端补齐 Coding Plan 额度查询服务（Kimi / 智谱 GLM / MiniMax）
  - `UsageScriptModal` 补上 `Token Plan` / `官方余额` 模板、自动识别、原生测试链路和卡片展示分流
  - 三语文案补齐 `usageScript.templateTokenPlan` / `usageScript.templateBalance` / `usageScript.tokenPlanHint` / `usageScript.balanceHint`
- 更新检查 / 版本提示
  - Web 后端新增 `GET /api/settings/latest-release`，统一查询 GitHub 最新 release 信息
  - Web 前端新增更新状态上下文，启动后自动轻量检查最新版本，并复用到 About 页与顶栏入口
  - About 页补齐“检查新版本”状态闭环：当前版本展示、最新版本检测、有更新时展示 release notes 摘要
  - 主界面标题栏补上更新提醒 badge，点击可直达设置页 About 标签
- 首选终端设置
  - Web 设置页补齐“首选终端”入口，支持按当前平台选择终端应用并即时保存
  - Web 后端设置模型补齐 `preferred_terminal`，并在按供应商打开终端时优先使用该设置
  - 若首选终端不可用，仍按各平台既有默认终端逻辑自动回退
- OpenCode / OpenClaw 健康检查
  - Web 后端补齐 OpenCode / OpenClaw 的流式健康检查分发逻辑，不再直接返回“暂不支持”
  - OpenClaw 支持按 `api` 协议分发到 OpenAI / Responses / Anthropic / Gemini 检查链路，并透传自定义 headers
  - OpenCode 支持按 `npm` SDK 包分发到对应检查链路，并补齐 `options.baseURL` / `options.apiKey` / headers 读取与默认端点回退
  - 对 AWS Bedrock、自定义认证头等当前仍无法无损构造请求的场景，返回明确错误提示
- Deep Link 远程配置合并
  - Web 端 Deep Link 导入改为“先解析 URL，再异步合并配置”，不再在解析阶段直接拒绝 `configUrl`
  - Provider deeplink 现支持通过 `configUrl` 拉取远程 `json` / `toml` 配置，并按桌面版既有规则补全 `apiKey` / `endpoint` / `model` / `homepage`
  - 远程配置拉取成功后会回填为本地 `config` 预览数据，导入确认弹窗可直接查看配置来源、URL 与合并后的配置内容

实现约束：

- 保持单 Agent 实现
- 未自动执行 build、全量测试、重型验证
- 文档与 README 三语同步更新

轻量自检：

- `git diff --check`
- 三语 i18n JSON 解析校验
