# CC Switch Web 化改造完整计划

## 1. 背景

当前仓库仍然是以 Tauri 桌面应用为中心的架构：

- 前端是 React + TypeScript
- 前端通过 `@tauri-apps/api` 调用本地能力
- 后端核心逻辑位于 Rust
- 入口、生命周期、托盘、深链接、自动更新、文件对话框等能力都依赖 Tauri

目标不是把项目改成纯云端 SaaS，而是改成下面这种运行方式：

- 前端是 Web UI
- 后端是本地 Rust 服务
- 浏览器访问 `http://localhost:xxxx`
- Linux 无桌面环境可直接运行
- Windows 可直接运行
- 支持 Docker 方式部署和启动

这意味着需要保留“本地服务能力”，但移除“桌面壳能力”。

## 2. 总目标

将项目从 “Tauri 桌面应用” 改造成 “Web 前端 + 本地 Rust 服务” 的形态，同时尽量保留现有核心业务逻辑。

最终交付应满足：

- 不依赖桌面窗口、托盘、系统 WebView 才能运行
- 在 Linux 服务器环境中可通过浏览器访问
- 在 Windows 环境中可直接运行本地服务并通过浏览器使用
- 提供 Docker 启动方式
- 保留核心能力：Provider、Settings、MCP、Prompts、Skills、Proxy、Workspace 相关核心逻辑

## 3. 非目标

以下内容不作为第一阶段目标：

- 兼容原有桌面端所有体验细节
- 同时长期维护 Web 版和 Tauri 版双入口
- 做成公网多租户 SaaS
- 在第一阶段保留系统托盘、自动更新、深链接、原生窗口拖拽等桌面特性
- 第一阶段解决所有远程访问安全问题

## 4. 架构原则

### 4.1 基本原则

- 保留 Rust 核心业务逻辑，避免重写已有配置管理和代理逻辑
- 前端只负责 UI 和交互，不直接绑定 Tauri
- 运行时能力通过统一适配层接入
- 桌面端能力和业务能力必须拆开，不能继续混在同一层
- 所有新增开发文档、计划文档优先放到 `docs-dev/`

### 4.2 目标架构

目标架构如下：

```text
Browser
  |
  v
Web UI (React + Vite)
  |
  | HTTP / SSE / WebSocket
  v
Local Rust Service
  |
  +-- Provider / Settings / MCP / Prompt / Skills / Workspace / Proxy
  +-- Local file system access
  +-- Local process / proxy management
  +-- SQLite / config storage
```

## 5. 现状拆分判断

### 5.1 可保留的核心

以下内容应优先保留并复用：

- `src-tauri/src/database/`
- `src-tauri/src/services/`
- `src-tauri/src/proxy/`
- 各类配置模型、Provider 逻辑、Settings 逻辑
- 前端现有页面结构、组件、状态组织方式

### 5.2 必须剥离的桌面耦合

以下内容不应继续作为运行前提：

- Tauri `invoke` / `listen`
- 托盘相关逻辑
- 单实例逻辑
- 深链接逻辑
- 自动更新逻辑
- 原生文件对话框逻辑
- 原生窗口拖拽区域逻辑
- 原生进程退出 / 重启逻辑

### 5.3 不能直接删除的部分

以下内容不能“直接删掉”：

- Rust 核心服务层
- 与 CLI 配置读写相关的本地文件逻辑
- 本地代理服务
- Workspace 文件访问能力

原因是这些能力在 Web 版仍然需要，只是调用方式要从 Tauri IPC 改为 HTTP API。

## 6. 文档目录调整策略

### 6.1 `docs/`

当前 `docs/` 下的桌面端历史文档、发布说明、图片引用、用户手册等内容不再适合作为当前改造阶段的主文档来源。

本轮处理策略：

- 直接清空 `docs/`
- 先不保留旧图片、旧 release notes、旧 user manual
- 后续只有在 Web 版重新形成稳定文档后，再决定是否重新启用 `docs/`

### 6.2 `docs-dev/`

`docs-dev/` 用于存放：

- 改造计划
- 技术路线
- 迁移记录
- 设计说明
- 阶段性执行文档

## 7. 阶段计划

## 7.0 当前执行进度（2026-03-31）

截至目前，已经完成的关键改造如下：

- 已建立前端运行时适配层，前端不再只依赖 Tauri 才能启动
- 已新增 Rust 本地 Web 服务入口，并支持托管 `dist/` 静态资源
- 已打通 `Settings`、`Providers`、`Proxy`、基础 `Failover` 的 HTTP API
- 已打通统一 `MCP` 的 Web API 主路径（列表、导入、编辑、删除、应用启用切换）
- 已打通 `Prompts` 的 Web API 主路径（列表、新增/编辑、删除、启用、读取当前提示词文件）
- 已打通 `Skills` 的首批 Web API 主路径（已安装列表、应用启用切换、卸载、备份列表）
- 已打通 `Skills` 的导入主路径（扫描未管理项、从应用目录导入）
- 已打通 `Skills` 的发现与安装主路径（仓库列表、仓库增删、发现可安装项、安装）
- 已打通 `Skills` 的备份恢复主路径（备份列表、恢复、删除）
- 已打通 `Skills` 的 Web ZIP 上传安装路径（批量选择、拖拽上传、多归档安装）
- 已打通 `Workspace` 的 Web API 主路径（工作区文件读写、Daily Memory 列表/搜索/编辑/删除）
- 已打通 `Sessions` 的 Web API 主路径（会话列表、消息详情、单个/批量删除）
- 已提供：
  - `pnpm dev` / `pnpm dev:web`
  - `pnpm build` / `pnpm build:d`
  - `pnpm start:web`
  - `pnpm start:w`
  - `pnpm start:l`
- 已增加 Docker 运行文件：
  - `Dockerfile`
  - `docker-compose.yml`
  - `.dockerignore`
- 已增加宿主机目录挂载示例：
  - `docker-compose.host.example.yml`
- 已增加 Linux 服务托管示例：
  - `deploy/systemd/cc-switch-web.service.example`
- 已增加 GitHub Actions Web CI：
  - `macOS / Windows / Linux` 跨平台静态校验
  - Docker 镜像构建与容器健康检查
- 已移除旧桌面端发布 workflow，当前仓库仅保留 Web 方向的 CI
- 已补充分发与启动体验文档：
  - `docs-dev/web-distribution-startup-plan.md`
- 已在 README 三语文件中补充本地运行和 Docker 运行说明
- 已在 Web 模式下收起一部分仍依赖桌面能力的入口，避免页面直接触发未迁移命令

当前已具备的 Web 主流程能力：

- 浏览器访问本地 Rust 服务
- Provider 基础增删改查与切换
- MCP 统一列表、导入、编辑、删除与启用切换
- Prompts 列表、编辑、删除与启用
- Skills 已安装列表、应用启用切换与卸载
- Skills 扫描未管理项与从应用目录导入
- Skills 仓库管理、发现列表与安装
- Skills 备份恢复与删除
- Settings 基础读写
- Proxy 启停、接管、配置读写
- Failover 基础队列与开关配置

当前仍未完成、需要继续迁移的重点：

- Skills
- Usage 统计
- OpenClaw 专属页面
- 整流器、全局出站代理等桌面设置区块
- Windows / Linux 直接运行的更完整包装方式（如脚本、服务化说明）

## 7.1 阶段一：运行时解耦

### 目标

让前端不再直接依赖 Tauri 作为唯一运行时。

### 工作项

- 新增统一运行时适配层
- 将前端中的 `@tauri-apps/api/*` 调用逐步抽离到适配层
- 为 Web 运行时实现 `fetch` 版本 API
- 识别并隔离桌面端专属交互
- 移除前端启动阶段对 Tauri 事件和 Tauri 退出机制的强依赖

### 产出

- 前端 API 访问不再散落在组件中
- 前端具备 Web 模式运行的基础能力

### 验收标准

- 前端可在浏览器环境启动而不是一启动就因 Tauri API 缺失报错
- 至少具备最小首页加载能力

## 7.2 阶段二：Rust HTTP 服务化

### 目标

将现有 Rust 核心逻辑通过 HTTP API 暴露出来。

### 工作项

- 新增 Web 服务入口
- 设计统一 API 路由
- 将现有 `commands/*` 背后的业务逻辑下沉或复用到 service 层
- 将 Tauri command 暴露方式替换为 HTTP handler 暴露方式
- 为需要状态推送的功能设计 SSE 或 WebSocket 机制

### 优先级高的 API 模块

- Providers
- Settings
- MCP
- Prompts
- Skills
- Proxy
- Workspace
- Sessions

### 验收标准

- 本地 Rust 服务启动后，前端可以通过 HTTP 正常获取和修改核心数据
- 不依赖 Tauri `invoke`

## 7.3 阶段三：Web 最小可用版本

### 目标

形成一个能在 Linux/Windows 下使用浏览器访问的最小可用版本。

### 工作项

- 让前端核心页面接入新的 Web API
- 下线无法在第一阶段支持的桌面专属功能入口
- 重新梳理设置页，区分：
  - 可在 Web 版保留的能力
  - 需要替换实现的能力
  - 暂时下线的能力

### 暂时应下线或禁用的能力

- 系统托盘
- 原生自动更新
- 窗口级拖拽行为
- 桌面端关闭最小化行为
- 原生文件选择器依赖强的交互
- 深链接

### 验收标准

- 浏览器可完成核心配置管理和代理相关操作
- 主流程不再依赖桌面窗口环境

## 7.4 阶段四：一键启动与部署

### 目标

提供你需要的三类运行方式：

- 本地开发一键启动
- Linux/Windows 直接运行
- Docker 运行

### 工作项

- 新增开发命令：
  - `pnpm dev`
  - `pnpm dev:web`
  - `pnpm build`
  - `pnpm build:d`
  - `pnpm start:web`
  - `pnpm start:w`
  - `pnpm start:l`
- 新增 Rust 服务启动命令
- 增加 Windows 启动脚本
- 增加 Linux 启动脚本
- 增加 Dockerfile
- 视需要增加 `docker-compose.yml`

### Windows 直接运行建议

- 启动 Rust 服务
- 服务监听本地端口
- 自动或手动打开默认浏览器访问 `http://localhost:xxxx`

### Linux 直接运行建议

- 支持前台启动
- 支持 `systemd` 方式托管
- 允许无桌面环境运行
- 默认监听 `127.0.0.1`，如需远程访问再显式配置监听地址

### Docker 运行建议

- 容器内运行 Rust 服务
- 提供前端静态资源
- 明确需要挂载的数据目录
- 明确需要挂载的宿主机配置目录

### 验收标准

- Linux 无桌面环境可启动服务并访问页面
- Windows 可直接启动服务并在浏览器打开
- Docker 可运行最小可用版本

## 7.5 阶段五：清理和稳定化

### 目标

删除已经不需要的桌面壳代码，减少双栈维护成本。

### 工作项

- 删除或归档 Tauri 桌面壳入口
- 清理无用依赖
- 清理前端桌面专属样式和逻辑
- 清理不再使用的测试 mock
- 梳理目录结构

### 验收标准

- 仓库主运行形态清晰
- 不再依赖桌面壳完成主功能

## 8. 拆分与删除清单

## 8.1 前端优先改造区域

- `src/main.tsx`
- `src/App.tsx`
- `src/lib/api/*`
- `src/lib/updater.ts`
- `src/components/settings/*`
- `src/hooks/useDirectorySettings.ts`
- 其他直接依赖 `@tauri-apps/*` 的模块

## 8.2 后端优先改造区域

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/*`
- 新增 Web API 入口模块

## 8.3 预期下线区域

- 托盘相关
- 深链接相关
- 单实例相关
- 自动更新相关
- 原生文件对话框相关
- 窗口生命周期相关

## 9. 兼容性策略

### 9.1 配置与数据

- 保留现有数据库和配置结构优先
- 避免为了 Web 化而重写存储格式
- 如果需要迁移，必须提供兼容策略和回滚策略

### 9.2 API 兼容

- 优先让 Web API 与现有前端调用模型保持接近
- 尽量减少页面层改动

### 9.3 平台兼容

- Windows、Linux 为第一优先级
- 无桌面 Linux 场景优先考虑
- macOS 暂不作为当前改造主目标

## 10. 风险清单

### 10.1 高风险

- 前端当前对 Tauri 的依赖非常深，拆分过程中容易出现连锁破坏
- 某些能力天然依赖本地系统集成，Web 化后需要替代方案
- Docker 与宿主机真实 CLI 配置之间有权限和挂载边界问题

### 10.2 中风险

- 原有测试体系基于 Tauri mock，后续需要同步调整
- 事件机制从 `listen` 改为 HTTP + SSE/WebSocket 后，前端状态同步逻辑会变化

### 10.3 低风险

- README、目录结构、开发文档可以逐步同步

## 11. 开发与交付建议

### 11.1 开发顺序

建议严格按下面顺序执行：

1. 文档与计划落地
2. 前端运行时解耦
3. Rust HTTP 服务骨架
4. 核心模块 API 接通
5. 一键启动与 Docker
6. 清理桌面壳
7. 补文档和验收

### 11.2 不建议的做法

- 不建议先大规模删除 `src-tauri`
- 不建议先删前端里的所有 Tauri 引用再想替代方案
- 不建议一开始就同时做全部模块迁移
- 不建议先做 UI 重构，再做运行时改造

## 12. 阶段性交付清单

### 里程碑 M1

- 前端在 Web 模式下能启动
- Rust Web 服务有基础健康检查接口

### 里程碑 M2

- Providers / Settings 可通过 HTTP API 工作
- 浏览器可以访问本地服务

### 里程碑 M3

- MCP / Prompts / Skills / Proxy 接通
- Linux 无桌面环境可运行

### 里程碑 M4

- Docker 可运行
- Windows/Linux 启动脚本完成

### 里程碑 M5

- 桌面壳代码完成清理
- 文档同步完成

## 13. 验收标准

满足以下条件才视为改造完成：

- 项目主运行方式不再依赖 Tauri 桌面窗口
- Linux 无桌面环境可运行
- Windows 可直接运行
- Docker 可运行
- 浏览器可访问本地服务
- 核心业务功能仍保留
- 文档目录清晰：
  - `docs/` 用于正式文档
  - `docs-dev/` 用于开发计划与迁移文档

## 14. 当前立即执行建议

基于当前状态，下一步最合理的执行内容是：

1. 继续迁移 MCP / Prompts / Skills 相关 API
2. 梳理并收敛 Web 模式下仍暴露的桌面专属入口
3. 补齐 Docker 使用细节和宿主机目录挂载策略
4. 视需要补充 Linux `systemd` / Windows 直接运行说明

不要先进行大规模删除，先完成“可替代运行路径”，再清理旧代码。
