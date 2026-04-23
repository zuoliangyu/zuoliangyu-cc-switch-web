# Web 端对齐计划（对齐 cc-switch v3.14.0）

## 背景

- 调查时间：2026-04-23
- 上游基线：`E:\zuolan_lib\cc-switch`，版本 `v3.14.0`，发布日期 `2026-04-21`
- Web 基线：`E:\zuolan_lib\cc-switch-web`，版本 `0.2.2`，发布日期 `2026-04-19`
- 上游本轮正式说明见 `E:\zuolan_lib\cc-switch\docs\release-notes\v3.14.0-zh.md`
- Web 端上一轮对齐记录见 `E:\zuolan_lib\cc-switch-web\docs-dev\web-parity-2026-04.md`

本文件用于回答两个问题：

1. `v3.14.0` 相对当前 Web 分支，到底多了什么
2. Web 分支下一步应该按什么顺序接入，避免一次性把复杂度拉爆

## 当前理解

当前 Web 分支并不是“落后很多个大版本的空壳”，而是已经补齐了上游在 `v3.13.x` 之前和 `v0.2.0` 发布线内的大量能力，包括：

- 模型拉取
- 官方额度展示
- Codex OAuth 托管认证
- Skills 更新与 `skills.sh` 搜索
- Deep Link 远程配置合并
- OpenCode / OpenClaw 健康检查
- 会话恢复终端与会话用量同步
- 多批第三方预设与合作伙伴展示

但 `v3.14.0` 的核心变化不是若干零散按钮，而是两条新的主线：

- 一条是 **Hermes Agent 成为第 6 个一等受管应用**
- 另一条是 **代理、会话、用量和命名体系继续演进**

这意味着当前任务不能按“补几个页面细节”的方式推进，而要按“分阶段迁移”推进。

## 已确认现状

以下事实已经通过本地代码与文档确认：

### Web 端当前仍是 5 应用模型

- `src/config/appConfig.tsx` 中仅有 `claude / codex / gemini / opencode / openclaw`
- `backend/src/app_config.rs` 中 `AppType`、`McpApps`、`SkillApps`、`PromptRoot` 等结构均未扩到 `hermes`
- `docs-dev/web-parity-2026-04.md` 中没有 `Hermes` 对齐记录

### 上游桌面端已经完成 6 应用扩容

- `src/config/appConfig.tsx` 已加入 `hermes`
- `src-tauri/src/app_config.rs` 已把 `hermes` 接入 `AppType`、`McpApps`、`SkillApps`、`PromptRoot`、`CommonConfigSnippets`
- `CHANGELOG.md` 和 `docs/release-notes/v3.14.0-zh.md` 已明确把 Hermes 定义为第 6 个受管应用

### Web 端尚未见到以下 v3.14.0 关键项

- `Hermes` 相关前后端能力
- `@tanstack/react-virtual` 会话列表虚拟化
- `Usage` 日期范围选择器与页码跳转
- `Local Routing` 命名统一
- `Routing` 激活时阻止切换到官方供应商
- `LemonData` 新预设

### 需额外核实但暂不作为首阶段阻塞项

- Claude Opus 4.7 预设矩阵和 pricing 刷新是否已经被其他提交部分吃进 Web 分支
- `Stream Check` 分类 toast 是否存在等价 Web 实现
- New API 用量脚本模板的 `User-Agent` 兼容修复是否已在 Web 端独立实现

## 迁移目标

本轮目标不是一次把桌面端 `v3.14.0` 全量照搬到 Web，而是：

- 先把 Web 与桌面端之间的**用户认知差异**收敛
- 再把 Web 缺失的**非 Hermes 核心能力**补齐
- 最后单独完成 **Hermes 全链路接入**

## 分阶段计划

### 第一阶段：非 Hermes 的低风险对齐

目标：先补“对用户可见、改动边界相对清晰、不会触发数据结构大迁移”的内容。

### 计划项

- 统一文案与命名：`Local Proxy Takeover` 收敛为 `Local Routing`
- 在 Routing 激活时阻止切换到官方供应商
- 会话列表虚拟化
- Usage 日期范围选择器与页码跳转
- 新增 `LemonData` 预设
- 同步必要的三语文案
- 在 `docs-dev/` 中补充阶段记录

### 影响范围

- 前端：
  - `src/components/proxy/*`
  - `src/components/providers/*`
  - `src/components/sessions/*`
  - `src/components/usage/*`
  - `src/i18n/locales/*`
- 后端：
  - 以现有 Web API 为主，尽量复用已有数据接口
- 文档：
  - `docs-dev/`
  - 若对外行为已稳定，再决定是否更新 README 三语

### 风险

- 命名统一可能波及现有 i18n key、toast 文案和说明文档
- 会话列表虚拟化需要注意高亮、展开、选中、滚动定位之间的交互回归
- Usage 日期范围若牵涉 SQL 或接口参数扩展，需要注意兼容已有筛选链路

### 验收标准

- UI 中面向用户的“Takeover”文案不再继续扩散，新增能力统一使用“Routing”
- Routing 激活时，官方供应商切换被明确拦截且提示原因
- 大量会话下滚动不再明显卡顿
- Usage 页面支持预设日期范围和自定义日期时间
- `LemonData` 在 Web 可创建、可展示、可保存

## 阶段记录

### 2026-04-23 已完成

- 对齐 `Local Proxy Takeover` 到 `Local Routing` 的文案与行为
- Routing 激活时拦截切换到官方供应商
- 会话列表引入虚拟化
- Usage 页面补齐日期范围选择器、起止时间、自定义范围与分页跳转
- 补齐 `LemonData` 在 Claude、Codex、Gemini、OpenCode、OpenClaw 的预设
- 同步三语合作伙伴文案与 Web 端图标资源

### 第一阶段状态

- 第一阶段计划项已完成
- 后续如继续推进，对齐重点转入第二阶段协议层能力，或单独立项处理 Hermes 全链路接入

### 第二阶段：代理与协议层对齐

目标：补齐 `v3.14.0` 中对 Web 后端最有价值的协议演进，但暂不引入 Hermes。

### 2026-04-23 已完成第一笔

- Claude Provider 表单已补齐 `gemini_native` 选项
- 三语文案已补齐 Gemini Native 的提示、完整 URL 说明与代理依赖提示
- Web 后端已引入 Gemini Native 所需的 URL 归一化、schema/请求转换、流式转换与 shadow 模块
- Claude 代理链路已支持 `gemini_native` 的识别、鉴权、目标 URL 重写、请求转换与响应回转

### 2026-04-23 已完成第二笔

- 前端 `Stream Check` 已补齐 `modelNotFound`、`rejected` 和 `httpHint.*` 分类提示
- 三语 locale 已补齐探测模型失效、请求被拒和常见 HTTP 状态的解释文案
- Web 后端 `Stream Check` 已返回 `errorCategory`
- 默认探测模型已刷新为 `gpt-5.4@low` 与 `gemini-3-flash-preview`
- Claude `gemini_native` 的健康检查已支持 URL 归一化、请求转换与错误分类回传

### 第二阶段状态

- `gemini_native` 的配置、请求转发与健康检查闭环已补齐
- `Stream Check` 分类反馈与默认探测模型刷新已补齐
- 第二阶段剩余工作可转入 Hermes 等后续能力

### 计划项

- `gemini_native` 代理能力
- 相关 schema / transform / streaming 适配
- 与现有 Claude Provider 表单中的 `apiFormat` 选项联动
- 补齐 `Stream Check` 分类反馈与默认探测模型刷新

### 影响范围

- 前端：
  - Claude Provider 表单
  - 错误反馈与测试入口
- 后端：
  - `backend/src/proxy/*`
  - 相关请求转换和流式转发逻辑

### 风险

- 这是协议层改动，测试面会明显大于第一阶段
- 如果没有足够守护测试，容易影响既有 OpenAI / Anthropic / Gemini 兼容链路

### 验收标准

- `gemini_native` 可被保存、可被识别、可走通请求转发
- 不影响现有 `openai` / `responses` / `anthropic` / `gemini` 路径
- `Stream Check` 能区分模型下架、请求被拒与常见 HTTP 错误

### 第三阶段：Hermes 全链路接入

目标：把 Web 从 5 应用扩到 6 应用，并引入 Hermes 的完整最小闭环。

### 2026-04-23 已完成第一笔

- Hermes 已先以第 6 个应用的基础骨架形式进入 Web 主界面
- App Switcher、可见应用设置和主界面默认回退逻辑已支持 Hermes
- 当前进入 Hermes 时会停在占位页，并显式避开现有 Provider 操作链路
- 后端本笔只补 `settings` 持久化字段与 `hermes_config_dir`，未扩 `app_config.rs`
- Hermes Provider、Memory、Session、MCP、Skills 等专属能力留待后续提交继续推进

### 2026-04-23 已完成第二笔

- Hermes 已补齐 Memory 面板入口，可在 Web 中切换 `MEMORY.md` 与 `USER.md`
- 后端已补 Hermes Memory 文件读写、限额读取与启用开关接口
- `config.yaml` 的 `memory` 段会被定向更新，避免覆盖其他 Hermes 配置段
- 当前仍未接入 Hermes Provider、Session、MCP、Skills，继续保持按闭环拆分推进

### 2026-04-23 已完成第三笔

- 设置页已补齐 OpenClaw 与 Hermes 的配置目录覆盖入口
- Web 本地服务已支持查询 Hermes 当前配置目录与默认配置目录
- Hermes Memory 相关文件与 `config.yaml` 的读写会跟随自定义目录覆盖生效
- 当前仍未接入 Hermes Provider、Session、MCP、Skills，继续保持按最小闭环拆分推进

### 2026-04-23 已完成第四笔

- Web 本地服务已补 Hermes `config.yaml` 的健康扫描接口
- Hermes 视图已接入配置告警 Banner，可提示 YAML 解析失败、默认模型缺失、重复 provider 等常见问题
- 三语 locale 已补齐 Hermes health 告警文案
- 当前仍未接入 Hermes Provider、Session、MCP、Skills，继续保持按最小闭环拆分推进

### 2026-04-23 已完成第五笔

- Hermes 占位页已补可用入口，可直接尝试打开 Hermes Web UI
- 本地服务已支持探测 Hermes Web UI 存活并返回可打开地址
- 若 Hermes Web UI 未启动，前端会弹出确认框，允许直接打开终端执行 `hermes dashboard`
- Hermes 配置告警 Banner 已补“到 Hermes Web UI 修复”跳转入口
- 三语 locale 已补齐 Hermes Web UI 相关文案

### 2026-04-23 已完成第六笔

- Web 本地服务已补 Hermes `model` 段读取接口
- Hermes 占位页已补只读的当前模型摘要，可展示 provider、default model、base URL、context length、max tokens
- 三语 locale 已补齐 Hermes model 摘要文案
- 当前仍未接入 Hermes Provider 编辑、Session、MCP、Skills，继续保持按最小闭环拆分推进

### 2026-04-23 已完成第七笔

- Web 本地服务已补 Hermes `custom_providers` 名称列表读取接口
- Hermes 占位页已补只读的已配置 Provider 列表展示
- 三语 locale 已补齐 Hermes live provider 列表文案
- 当前仍未接入 Hermes Provider 编辑、Session、MCP、Skills，继续保持按最小闭环拆分推进

### 第三阶段当前状态

- 第三阶段已完成七笔闭环：基础承载面、Memory 面板、配置目录覆盖入口、配置健康告警、Web UI 入口、model 摘要只读面板、live provider 列表只读展示
- 当前已具备 Hermes 的应用入口、可见性控制、流程保护、Memory 基础闭环、自定义目录承接、配置告警提示、Web UI 跳转/启动引导、model 摘要只读展示与 live provider 列表只读展示
- 后续提交应继续按最小闭环拆分，不把 Provider、Memory、Session 混成一笔

### 计划项

- 应用枚举扩容：
  - `AppId`
  - `APP_IDS`
  - `MCP / Skills / Prompt / Settings` 相关结构
- 设置项与目录覆盖：
  - `hermesConfigDir`
  - 首选终端联动
- Hermes Provider：
  - 表单
  - 预设
  - additive mode 状态
  - 只读 `providers:` dict 条目展示
- Hermes Memory：
  - `MEMORY.md`
  - `USER.md`
  - 启用开关
  - 限额提示
- Hermes MCP / Skills 接入
- Hermes Session 接入
- Hermes 健康检查与 Web UI / Dashboard 跳转
- Deep Link 的 Hermes 语义
- 数据库与配置迁移策略

### 影响范围

- 前端几乎覆盖主工作台：
  - App 切换
  - Provider
  - Settings
  - Skills
  - MCP
  - Sessions
  - Prompt / Memory 入口
  - i18n
- 后端几乎覆盖配置与同步主链路：
  - `app_config`
  - provider service
  - session service
  - health check
  - settings
  - deeplink
  - 数据库迁移

### 风险

- 这是本轮最大风险项，不适合和第一阶段混做
- Hermes 是 YAML + additive mode，不同于现有 Web 分支已熟悉的 JSON / TOML + 覆盖式切换
- `mcp_servers` / `skills` 的 `enabled_hermes` 涉及 schema 变更，必须谨慎处理从 Web `schema v8` 演进的路径
- 如果没有 round-trip 守护，容易破坏用户手写的 Hermes YAML 字段

### 验收标准

- Web 可把 Hermes 作为第 6 个应用稳定展示
- Hermes provider、memory、mcp、skills、session 至少形成“可读 + 可写 + 可切换/启用”的最小闭环
- 对用户手写 YAML 的回写不丢未知字段，不破坏 OAuth MCP `auth`

## 暂不纳入 Web 首轮迁移的项

以下内容不建议在本轮一并推进：

- Tauri 窗口控件
  - Web 分支没有桌面壳，这项没有直接照搬价值
- Linux Wayland 原生窗口交互修复
  - 属于桌面容器问题，不是当前 Web 分支主矛盾
- 上游 README 赞助区与桌面分发说明
  - 只有当 Web 自身对外描述确实变化时才同步三语 README

## 建议的落地顺序

1. 先完成第一阶段文案与页面能力对齐
2. 第一阶段稳定后，再推进第二阶段协议层
3. 最后单独拉出 Hermes 专项

不建议直接从 Hermes 开始，原因如下：

- 一上来就会碰应用枚举、schema、配置读写、Session、MCP、Skills 多线改动
- 难以快速得到稳定可验收的中间成果
- 容易把“上游在演进”和“Web 在补课”两种复杂度叠在一起

## 本次文档结论

`v3.14.0` 相对当前 Web 分支的真正增量，核心是：

- `Hermes` 全链路接入
- `Local Routing` 语义统一
- 会话与用量页继续工程化升级
- 代理协议继续扩展到 `gemini_native`

因此，后续实施建议按以下原则执行：

- 先小步收敛用户可见差异
- 再处理协议层
- 最后做 Hermes 这条重线

## 已完成记录（2026-04-23）

第一阶段已先落下两个低风险对齐项：

- 已将前端面向用户的 `Takeover / 接管 / テイクオーバー` 文案统一收口为 `Routing / 路由 / ルーティング`
- 已补上“Routing 激活时阻止切换到官方供应商”：
  - 前端在供应商切换动作前直接拦截并提示原因
  - 后端 `switch` 服务同步加入防御，避免绕过前端
- 已补上会话详情区消息列表虚拟化：
  - 引入 `@tanstack/react-virtual` 以降低长会话滚动开销
  - 长消息默认折叠；当搜索命中折叠内容时自动展开
- 第一阶段又新增完成一项：

- 已对齐 Usage 日期范围选择器与请求日志分页跳转：
  - 前端统一从 `days + rolling/fixed` 迁移到 `UsageRangeSelection`
  - 新增预设范围：`today / 1d / 7d / 14d / 30d / custom`
  - 新增日历+时间选择器，支持在总览与请求日志筛选中共用
  - 请求日志分页新增页码输入跳转
  - Web runtime / Web API / Rust service 已同步为 provider/model stats 透传 `startDate/endDate`
- 已补齐 `LemonData` 预设：
  - Claude、Codex、Gemini、OpenCode、OpenClaw 五条配置链路已对齐
  - 三语合作伙伴文案已补齐
  - Web 端本地图标资源与映射已接入

## 后续执行入口

当前第一阶段已完成，后续建议按计划继续：

1. 继续第二阶段剩余项，补 `Stream Check` 分类反馈与默认探测模型刷新
2. 第二阶段收口后，单独立项推进第三阶段 Hermes 全链路接入

## 备注

- 本文件是迁移计划，不代表上述能力已在 Web 端完成
- 在实际代码落地后，如涉及页面行为、接口、配置或数据结构变化，需要同步补充对应文档记录
- 若后续修改 README 任一语言版本，必须同步检查 `README.md`、`README_ZH.md`、`README_JA.md`
