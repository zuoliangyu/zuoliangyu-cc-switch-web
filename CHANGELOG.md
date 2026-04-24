# Changelog

本仓库从 Web 分支独立维护开始，重新以 `0.1.0` 作为初始版本。

## [0.3.0] - 2026-04-24

### 数据库 schema

- schema 版本从 `v8` 升到 `v10`，与上游 `cc-switch` 3.14 系列对齐；新增 `v8 -> v9` 模型定价种子刷新迁移与 `v9 -> v10` Hermes 支持列迁移，解决共享 `~/.cc-switch/cc-switch.db` 时被上游升到 `v10` 后 Web 端启动报 `数据库版本过新（10），当前应用仅支持 8` 的问题
- `mcp_servers` / `skills` 两表新增 `enabled_hermes` 列；后端 `McpApps` / `SkillApps` 同步补 `hermes` 字段，DAO 的 SELECT / INSERT / UPDATE 全部读写新列，从数据库起到前后端类型完全对齐 `hermes`
- 迁移回归测试由 `schema_migration_v7_to_v8_compatibility_version_only` 改写为 `schema_migration_from_v7_preserves_skills_columns`，校验从 `v7` 起一路迁移到当前 `SCHEMA_VERSION` 时既保留既有列也正确落下 `enabled_hermes`

### Provider、预设与界面对齐

- Claude / OpenClaw / OpenCode 三端直连 Moonshot 的预设从 `kimi-k2.5` 升到 `kimi-k2.6`
- Codex 预设新增 DDSHub 条目，与上游合作伙伴布局一致
- `ProviderIcon` 在图标、回退首字母以及远端图片三种渲染路径上都补上 `title={name}`，悬停始终能看到供应商名称
- `useAutoCompact` 的 `normalWidthRef` 写入移入 overflow 分支，修复最大化后还原窗口无法重新进入紧凑模式的粘死问题
- 工具栏里所有 ghost 图标按钮统一加 `w-8 px-2`，多 App 切换时宽度不再跳动
- `ScrollArea` 视口追加 `[&>div]:!block [&>div]:!min-w-0 [&>div]:!w-full`，根布局加 `pb-4`，改善会话列表在滚动容器内的对齐与底部留白
- `UsageScriptModal` 的 `getProviderCredentials` 识别 Hermes（snake_case）与 OpenClaw（camelCase）两种扁平 `settingsConfig`，BALANCE / TOKEN_PLAN 分支改为复用 `providerCredentials`

### 代理与会话

- 后端 `session_manager/providers/gemini.rs` 读取每个会话目录下的 `.project_root`，把项目路径回填到 `SessionMeta.project_dir`，与上游 `gemini cli resume` 行为对齐
- `proxy/handlers.rs` 的 `should_use_claude_transform_streaming` 在 `codex_oauth + openai_responses` 组合下强制返回 `true`，即便客户端未请求流式、上游非 SSE 也会走 Claude 流式转换路径

### 类型与面板

- `AppId` 体系内 `hermes` 覆盖更完整：`APP_IDS`/`MCP_SKILLS_APP_IDS` 相关记录值、`McpApps`/`SkillApps`/`ProvidersByApp`/`CurrentProviderState` 等类型，以及 `McpFormModal`、`UnifiedMcpPanel`、`UnifiedSkillsPanel`、`deeplink/importer.ts`、`tests/msw/state.ts` 中的硬编码构造点，全部补齐 `hermes` 分支
- `HermesPlaceholderPanel` 的 `providerId` 补上 `string` 类型
- 前端 `providersApi` 新增 `getHermesLiveProviderIds`，与 `useHermes` hook 对接

### 构建与工程化

- `backend/src/proxy/sse.rs` 补齐 `take_sse_block` 与 `append_utf8_safe`（此前 `streaming_gemini.rs` 有 import 但没有实现），解决 `cargo check` 的 `E0432` 报错
- `backend/Cargo.toml` 在 `reqwest` feature 列表里加上 `blocking`，修复 `commands/hermes.rs` 中 `reqwest::blocking::Client` 的 `E0433`
- `backend/src/proxy/response_processor.rs` 的 `build_state` 测试辅助补齐 `gemini_shadow` 字段，解决 `E0063`
- `backend/src/proxy/providers/claude.rs` 去掉 `AuthStrategy` 下已被前面分支完全覆盖的 `_ => vec![]` 分支，清除 `unreachable_pattern` 警告
- `backend/src/proxy/providers/gemini.rs` 的 `parse_oauth_access_token` 被测试引用，改为 `#[allow(dead_code)] pub fn` 留存，既消除 `dead_code` 警告也不破坏测试
- `scripts/dev.ps1` 把 `-Arguments @($Mode) + $ExtraArgs` 改成表达式形式 `(@($Mode) + $extras)`，并在无额外参数时 fallback 到空数组，避免 PowerShell 把 `+` 当位置参数、以及 `Start-Process` 拒收含 `$null` 的 `-ArgumentList`
- `package.json` 加 `pnpm.overrides.baseline-browser-mapping: ^2.10.21`，解决每次 `vite` 启动打印的 `data in this module is over two months old` 警告
- 更新检测入口 `WEB_GITHUB_REPO` 由 `zuoliangyu/zuoliangyu-cc-switch-web` 更正为 `zuoliangyu/cc-switch-web`，不再依赖 GitHub 301 重定向

### 文档与版本

- 仓库版本提升到 `0.3.0`
- 默认 README 改为中文：`README.md` 现在是中文版本，英文内容迁移到 `README_EN.md`，同时删除旧的 `README_ZH.md`；三份 README 的语言切换行、`AGENTS.md` 与 `docs-dev/web-parity-v3.14.0-plan-2026-04.md` 中的命名描述一并更新
- `README.md` / `README_EN.md` / `README_JA.md` 同步更新 `0.3.0` 版本说明

## [0.2.2] - 2026-04-19

### 修复

- 将当前 schema 版本正式提升到 `v8`，补齐缺失的 `v7 -> v8` 兼容迁移，避免 `v0.2.0` 这一线发布包在数据库启动时卡在 `未知的数据库版本 7，无法迁移到 8`
- 补充 `v7 -> v8` 兼容迁移回归测试，确保数据库能从上一版本平滑升级到当前 schema

### 文档与版本

- 仓库版本提升到 `0.2.2`
- README、README_ZH、README_JA 同步更新 `0.2.2` 版本说明与 schema `v8` 兼容迁移说明

## [0.2.1] - 2026-04-19

### 修复

- 修复 `v0.2.0` 发布包误将数据库 schema 版本提升到 `v8`、但未补齐最后一步迁移的问题，避免已有数据库或新数据库在启动时卡在 `未知的数据库版本 7，无法迁移到 8`
- 为数据库迁移补充回归测试，覆盖“当前 schema 从上一版本升级到最新版本”这条链路，防止再次出现仅提升版本号却遗漏最后一步迁移的发布事故

### 文档与版本

- 仓库版本提升到 `0.2.1`
- README、README_ZH、README_JA 同步更新 `0.2.1` 版本说明与本次发布修复内容

## [0.2.0] - 2026-04-11

### Provider、认证与预设能力对齐

- 补齐 Codex OAuth 托管认证闭环、多账号文案与 Responses 协议约束
- 补齐 Gemini 官方 OAuth 判断、Claude 预设隐藏支持、Claude Thinking 回退展示与 adaptive thinking 到 `xhigh` 的映射修正
- 补齐 Web 版 Provider Key 锁定逻辑、Key 编辑重命名闭环、按供应商打开终端，以及 OpenCode / OpenClaw 健康检查与测试入口
- 补齐 additive provider live 管理标记、累加模式复制仅落库行为、Provider 卡片状态展示与动作限制
- 补齐 OMO 提示文案、OMO Slim 高级字段提示与 OMO Slim Council agent
- 补齐 DDSHub、LionCCAPI、Shengsuanyun、TheRouter、PIPELLM 等预设、预设图标资源与合作伙伴标识/促销链路
- 对齐 Oh My OpenCode 预设地址、E-FlowCode 预设默认密钥、Provider 预设展示顺序与 X-Code 预设图标键
- 修正 Anthropic 转 OpenAI 的 system 消息归一化逻辑，并恢复 OpenCode 模型拉取与通用配置迁移兼容

### 用量、设置与工作流补全

- 恢复用量页会话同步与数据来源概览，补齐请求日志来源列、应用过滤联动与用量页应用类型过滤
- 补齐原生余额与 Token Plan 模板、Token Plan 内联徽章、官方额度当前态语义，以及 GitHub Copilot 额度展示
- 补齐本地服务开机自启、首选终端设置、Claude Code 插件自动同步、首次安装确认跳过与首次使用提示
- 补齐更新检查与版本提示、认证中心/设置页/用量页三语文案，以及模型拉取、认证标签等多语言补全

### Skills、Session 与 Deep Link 对齐

- 补齐 skills.sh 搜索能力、Skills 更新能力与 Skill 存储位置切换
- 补齐会话搜索高亮、会话恢复终端，以及验证码复制兼容性
- 补齐 Deep Link 远程配置合并、Provider 预览细节、配置预览语义化、用量配置预览与确认提醒
- 补齐 Deep Link skill 导入提醒、mcp 预览摘要、解析失败提示、子资源标题展示，以及配置合并失败时的降级导入行为

### 资源、图标与辅助体验

- 补齐本地图标元数据搜索与 Web 版预设图标资源
- 补齐通用配置编辑引导、通用配置弹窗引导与 OMO Slim 相关提示文案

### Web 界面升级

- Provider 与 Settings 页面升级为工作台式信息层级，重构顶部引导区、分区卡片与粘性操作区
- Skills 与 Sessions 页面升级为统一的玻璃卡片工作台风格，补强筛选区、空状态、列表卡片和详情区层次
- Skills 仓库管理面板、会话目录面板与通用全屏面板同步切换到新的 Web 视觉语言

### 文档与版本

- 仓库版本提升到 `0.2.0`
- README、README_ZH、README_JA 同步更新当前版本与最近完成的 Web 能力/UI 升级说明
- 补充 `0.2.0` 发布说明，归档 `0.1.3` 之后到当前版本之间的全部提交范围

## [0.1.3] - 2026-04-05

### Web 能力对齐

- 为 Claude、Codex、Gemini、OpenClaw 的供应商表单补齐模型拉取能力
- 补齐 Claude、Codex、Gemini 的官方订阅额度展示与查询链路
- 为 Web 本地服务补齐环境变量冲突检测、删除与恢复接口，并在前端增加冲突提醒条
- 为 Web 端增加 Deep Link 导入能力，支持 `?deeplink=...` 自动导入与手动粘贴 `ccswitch://...`
- About 页面增加“检查新版本”入口，直接跳转到 GitHub 最新发布页

### 文档

- README 中补充本轮已对齐的 Web 能力说明
- 新增 `docs-dev/web-parity-2026-04.md` 记录本轮对齐范围与约束

## [0.1.2] - 2026-04-05

### 界面视觉

- 将全站基础主题切换为 Material Monet 风格配色，重写 light / dark 下的核心主题变量
- 调整全局玻璃卡片、页面背景层次和焦点高亮，统一为更柔和的 Monet 视觉语言
- 替换按钮、标签页、输入框、开关、首页应用标签、供应商卡片与设置页关键状态的硬编码蓝色主色，避免旧主题残留
- 设置页新增 Material Monet 主题方案选择，支持多套预置配色卡片并与浅色 / 深色 / 跟随系统组合使用

### 运行时与端口

- 为本地 Web 服务增加 `--host`、`--backend-port` 与 `--port-scan-count` 启动参数，环境变量 `CC_SWITCH_WEB_HOST / PORT / PORT_SCAN_COUNT` 继续兼容
- 发布态服务默认首选端口调整为 `8890`，当端口被占用、被系统排除或无权限绑定时，会自动向后尝试可用端口
- 修复启动日志先打印 `listening` 再实际绑定端口的误导行为，改为绑定成功后再输出最终监听地址
- 为 `pnpm dev` 增加 `-f/--frontend-port`、`-b/--backend-port` 与 `--host` 参数，前端与后端端口选择逻辑统一
- 更新 Docker 默认端口与 compose 映射方式，支持通过 `CC_SWITCH_WEB_PORT` 统一指定容器内外监听端口，并默认关闭容器内自动换端口以避免端口映射漂移

## [0.1.1] - 2026-04-03

### 修复

- 修复 Web 模式下 Skills 卸载与应用开关在 repo 型 skill id 含 `/` 时的请求链路问题
- 为 Skills 相关 Web 请求补充回归测试，覆盖 repo 型 skill id 的卸载与开关场景
- 修复本地开发模式可能误命中旧 `dist` 静态资源的问题，默认禁用后端静态前端托管，避免 `3000` 与 `8788` 混用导致排查失真

### 开发体验

- 为本地 `pnpm dev` 增加前端请求/响应调试日志
- 为本地 Rust Web API 增加 method/path/status/耗时日志，便于定位请求链路问题
- 更新中英日 README，本地开发文档同步补充调试日志与访问入口说明

### 兼容性与运行时

- 引入 `env_logger` 初始化后端日志输出，便于本地开发和问题定位
- 保持发布版默认不启用本地开发注入的请求调试开关

## [0.1.0] - 2026-04-02

### 首次发布

这是 `CC Switch Web` 仓库独立维护后的首个正式版本。

当前版本不再延续旧桌面端发布线，而是以 Web-only 形态重新建立 `0.1.0` 基线，定位为：

- 前端：浏览器 Web UI
- 后端：本地 Rust 服务
- 访问方式：浏览器打开本地地址
- 支持场景：Windows、macOS、Linux、无桌面的 Linux 服务器、Docker

### 仓库定位与版本基线

- 正式建立 `cc-switch` 的 Web 分支仓库定位
- 仓库包名、项目名称、作者信息、仓库地址与说明文档统一切换到 `cc-switch-web`
- 清理继承自旧桌面分支的历史发布语义，以 `0.1.0` 作为当前仓库首发版本
- README、CHANGELOG 与仓库元信息同步收敛到 Web-only 口径

### 架构调整

- 完成从桌面壳架构向「Web 前端 + 本地 Rust 服务」架构的主线收敛
- Rust 服务支持直接托管前端静态资源，发布产物可作为单文件嵌入式 Web 服务运行
- 前端主流程不再以桌面运行时为前提，核心交互统一面向本地服务 API
- 默认数据路径保持与 CC Switch 本地端一致，继续使用 `~/.cc-switch`

### 核心功能迁移

本版本已将当前 Web 端可用主流程整理为正式发布基线，涵盖：

- Provider 配置管理、切换、导入、健康检查、排序与通用配置能力
- MCP 配置管理、导入、编辑、删除、启用切换与同步相关能力
- Prompt 管理、读取、编辑、删除与启用能力
- Skills 的扫描、导入、安装、卸载、仓库管理、备份恢复与统一管理能力
- Workspace、Session、Usage 统计等核心页面能力
- Proxy、Failover、WebDAV Sync、数据库导入导出、备份等本地服务能力
- OpenCode、OpenClaw、Claude、Codex、Gemini 等当前 Web 主路径下的配置接入能力

### 运行与分发

- 提供统一的开发、构建、检查入口：
  - `pnpm dev`
  - `pnpm build`
  - `pnpm check`
- 提供 Windows PowerShell 对应入口：
  - `scripts/dev.ps1`
  - `scripts/build.ps1`
  - `scripts/check.ps1`
- 新增 Windows 本地导出脚本 `scripts/package-artifacts.ps1`
  - 可一次生成 Windows 可执行文件、Linux 发布包、Docker 镜像包
- Linux 发布链调整为 `x86_64-unknown-linux-musl`，尽量减少宿主机运行库差异导致的问题
- Docker 运行模式与 Linux 发布包导出链路已纳入正式支持范围
- 提供 Linux `systemd` 示例，便于无桌面服务器长期托管

### 工程化与 CI/CD

- 新增并收敛脚本体系，仅保留 `dev / build / check` 为主入口
- 脚本输出与错误提示统一为英文，降低跨平台使用和日志排查成本
- GitHub Actions 已覆盖：
  - Web 检查
  - 平台包构建
  - Docker 镜像构建
- Linux 打包链统一通过 Docker 多阶段构建导出
- 增加本地与 CI 复用的检查脚本，统一前端与 Rust 静态检查流程

### 清理与收口

- 删除旧桌面端相关的无效脚本、发布口径和残留说明
- 清理与 Tauri / 桌面壳强耦合的仓库结构、文案与部分旧兼容逻辑
- 将当前仓库明确收敛为 Web-only 维护方向，不再以桌面 GUI 发布为目标
