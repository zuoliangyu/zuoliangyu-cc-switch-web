# Web 化剩余缺口审计

更新时间：2026-04-01（完成 Web 兼容 API 收口后）

## 结论

当前仓库的 Web 主业务能力已经基本拉齐，且前端仍在使用的旧兼容命令链路也已经补齐到本地 Rust HTTP API。

补充说明：

- 本地运行的 Web 版不是“只存在浏览器里的临时前端”
- 本地 Rust 服务默认仍然使用 `~/.cc-switch` 作为 CC Switch 自身数据目录
- 也就是说，本地 Web 模式下的 `config.json`、`settings.json`、`cc-switch.db`、备份、Skills SSOT 等默认与原本地端保持同一路径策略
- Docker 运行是例外：默认写入容器 volume，而不是宿主机 `~/.cc-switch`

剩余未迁移项现在主要分为两类：

1. 仍建议继续处理，但以“收尾/重构/兼容收口”为主，而不是核心业务缺失
2. 明确属于桌面端/Tauri 壳能力，建议直接删除、隐藏或保留为仅 Tauri 生效，不再做 Web 映射

换句话说：

- `Providers / Proxy / Failover / Settings / MCP / Prompts / Skills / Workspace / Sessions / Usage / OpenClaw / Universal Provider / OMO / OMO Slim` 的 Web 主路径已基本可用
- 当前更大的问题已经不是“缺少 Web 核心功能”，而是“仓库里还残留不少桌面 API 和旧兼容层”

## 一、仍建议继续处理

| 项目 | 当前状态 | 建议动作 | 原因 |
|---|---|---|---|
| 旧兼容 Skills API | 工具层仍保留 | 逐步移除对旧别名的依赖 | 当前实际主路径已走统一 Skills API，不宜长期双轨 |
| 旧兼容 MCP API | 工具层仍保留 | 收敛到统一 MCP API | 当前 Web 主路径已完成统一 MCP 管理，这批兼容命令应减量 |
| 未使用的旧前端 API 封装 | 已继续收敛 | 继续按调用点清理 | 目录打开、文件对话框、ZIP 本地路径安装等前端残留已开始删除 |
| 未接入页面的路径查询命令 | 仅工具层存在 | 按实际入口决定保留或清理 | 如 `get_claude_code_config_path` / `get_app_config_path` 当前没有 Web 页面主链依赖 |

## 二、建议直接删除或仅保留 Tauri

| 项目 | 当前状态 | 建议 | 原因 |
|---|---|---|---|
| `restart_app` | Web 未映射 | 删除 Web 依赖，保留 Tauri 或清理 | Web 本地服务模式不需要应用重启入口 |
| `check_for_updates` | Web 未映射 | 直接保持桌面专用 | 自动更新属于桌面壳能力 |
| `set_auto_launch` / `get_auto_launch_status` | Web 未映射 | 保持桌面专用或删除 | 开机自启不是浏览器产品能力 |
| `is_portable_mode` | Web 未映射 | 保持桌面专用或删除 | 属于桌面分发形态判断 |
| `pick_directory` | Web 未映射 | 不再迁移 | 浏览器不能可靠调用本地原生目录选择器，当前已改为手填路径 |
| `save_file_dialog` / `open_file_dialog` | Web 未映射 | 不再迁移 | Web 已改用上传/下载方案 |
| `open_workspace_directory` | Web 未映射 | 不再迁移 | Web 页面已降级为复制路径 |
| `open_provider_terminal` | Web 未映射 | 不再迁移 | Web 分支已移除该入口，避免依赖桌面终端 |
| `launch_session_terminal` | Web 未映射 | 不再迁移 | Web 分支统一复制恢复命令，不再拉起终端 |
| `open_app_config_folder` / `open_config_folder` | Web 未映射 | 不再迁移 | 与本地 GUI 文件管理器强耦合 |
| `apply_claude_plugin_config` | Web 未映射 | 建议下线或桌面专用 | 属于旧桌面集成逻辑 |
| `apply_claude_onboarding_skip` / `clear_claude_onboarding_skip` | Web 未映射 | 建议下线或桌面专用 | 属于 Claude 桌面集成兼容逻辑 |
| 单实例/窗口/托盘相关后续残留 | 未纳入 Web | 建议继续清理 | 与目标架构不一致 |

## 三、已降级处理，不应再算作“Web 功能缺失”

这些能力在 Web 下不是“没做”，而是已经做了有意识的替代方案：

| 原桌面能力 | Web 当前方案 |
|---|---|
| 打开工作区目录 | 复制目录路径 |
| 打开 Session 终端 | 复制恢复命令 |
| 打开 Provider 专属终端 | Web 分支移除该入口 |
| 原生文件导入 | 浏览器上传 SQL |
| 原生文件导出 | 浏览器下载 SQL |
| 原生目录选择器 | 手动填写目录覆盖 |
| 桌面自动更新 | Web 下隐藏 |

当前已完成的收口：

- Provider 通用配置片段的读取、写入、提取能力已通过本地 Rust HTTP API 暴露给 Web 版，不再依赖 Tauri `invoke` 直连
- 全局出站代理的旧兼容接口（读取 URL、保存 URL、测试、扫描、运行态状态）已补齐到 Web 本地服务
- 设置保存后的 `sync_current_providers_live` 兼容链路已补齐到 Web 本地服务，目录变更后可继续回写 live 配置
- 顶栏更新徽标已从 Web 分支前端移除
- About 页已移除桌面自动更新与便携模式展示
- Web 模式下不再持有前端自动更新上下文
- OMO / OMO Slim 已补齐本地文件读取、当前配置读取与停用的 Web 路径
- Deeplink 前端导入对话框与 Web 入口已从当前分支移除
- Deeplink Rust 模块、Tauri 命令、协议插件与 macOS URL scheme 已从当前分支移除
- 前端已不可达的桌面设置联动已移除，包括自动重启、自启动和 Claude 插件/Onboarding 同步分支
- 前端托盘菜单刷新 API 已在 Web 运行时收口为 no-op，避免继续误调用桌面壳能力
- 桌面自动更新与便携模式检测的 Rust 命令、Updater 插件依赖已从当前分支移除
- 已无前端入口的 `open_config_folder` / `open_app_config_folder` 命令已从当前分支移除
- Provider 专属终端与 Session 终端拉起命令已从当前分支移除，统一降级为复制操作
- Provider 预设中的合作/促销展示已从 Web 分支前端移除，不再在预设选择、配置表单和 Provider 卡片中显示营销信息
- Tauri 托盘、单实例、轻量模式、关闭最小化到托盘与相关前端刷新兼容调用已从当前分支移除
- 默认桌面 `main.rs` 入口、Windows 覆盖配置、`Info.plist`、`wix` 目录和 Common Controls 清单已从当前分支移除
- 旧桌面 `run()` 启动器、`tauri-build`、`tauri.conf.json`、`tauri-plugin-log/process` 已从当前分支移除
- `appConfigDir` 覆盖路径已改为本地 Rust 服务写入独立文件，不再依赖 Tauri Store
- 原生目录选择器、原生文件对话框、系统链接/目录打开命令已从当前分支的 Rust 侧移除，Web 模式统一走手填、上传下载或复制路径
- 前端设置页已收敛为浏览器上传/下载 SQL；工作区和 Daily Memory 页面统一改为复制目录路径；Skills ZIP 安装统一走浏览器多文件上传与拖拽
- Skills 顶部工具栏已直接提供 ZIP 上传入口；环境变量冲突治理横幅已从 Web-only 主界面移除
- 旧的 Tauri 命令包装层已继续瘦身，已删除一批无前端入口的 MCP / Skills / 导入导出 / 路径查询兼容命令及对应测试 mock
- 已删除仅供旧桌面环境冲突治理使用的 env 命令与服务模块，避免继续保留无入口系统级能力
- `ProxyService -> ProxyServer -> RequestForwarder` 的核心链路已改为显式注入 `copilot_auth_state`，不再通过 `AppHandle.state()` 读取容器状态
- Failover 热切换与 WebDAV 自动同步的服务层逻辑已去掉 `AppHandle` 依赖，当前 Web-only 主链不再依赖 Tauri 事件才能运行
- 命令层的故障转移与 Universal Provider 同步事件已从当前分支移除，前端同步改为显式请求/缓存失效而非依赖 Tauri event bus
- Web 启动入口已重新接回 WebDAV auto sync worker，数据库变更现在能真正触发本地后台自动同步
- Web 启动入口已接回周期备份/维护检查，旧备份策略配置重新对 Web 运行模式生效
- OMO 仅保留 Web 正在使用的“读取本地配置填充表单”路径，旧的 Rust 侧直接导入生成 provider 逻辑已移除

## 四、基于前端命令差集的剩余项

以 `src` 中实际 `invoke(...)` 调用与 Web runtime 映射做差后，当前剩余未映射命令主要如下：

| 类别 | 命令 | 当前判断 | 建议 |
|---|---|---|---|
| 环境变量治理 | `check_env_conflicts` / `delete_env_vars` / `restore_env_backup` | Web 主界面入口已移除，不属于本地 Web 服务主链 | 保持不映射，并继续清理残余运行时代码 |
| 桌面对话框 | `pick_directory` / `open_file_dialog` / `save_file_dialog` / `open_zip_file_dialog` | Web 前端主路径已不再依赖 | 不再迁移，并继续清理残余封装 |
| 桌面目录打开 | `open_workspace_directory` | Web 前端主路径已不再依赖 | 不再迁移，并继续清理残余封装 |
| 旧 MCP API | `get_claude_mcp_status` / `read_claude_mcp_config` / `upsert_claude_mcp_server` / `delete_claude_mcp_server` / `get_mcp_config` / `upsert_mcp_server_in_config` / `delete_mcp_server_in_config` / `set_mcp_enabled` / `validate_mcp_command` | 当前页面主路径已走统一 MCP API，差集里这批主要是旧封装残留 | 确认无入口后删除旧封装 |
| 旧 Skills API | `get_skills` / `get_skills_for_app` / `install_skill` / `install_skill_for_app` / `uninstall_skill` / `uninstall_skill_for_app` / `install_skills_from_zip` | 当前页面主路径已走统一 Skills API；前端 ZIP 安装已统一成上传归档 | 确认无入口后继续清理运行时残留 |
| 旧导入导出文件路径 API | `export_config_to_file` / `import_config_from_file` | Web 前端主路径已不再依赖 | 保留运行时兼容时再评估，前端不再迁移 |
| 未接入页面的路径查询 | `get_claude_code_config_path` / `get_app_config_path` | 当前 Web 页面未使用 | 评估后删除或继续仅保留桌面 |

换句话说，当前剩余差集已经不再是“Web 核心功能缺失”，而是：

1. 桌面专属能力的有意识不迁移
2. 旧 API 别名和死代码的收口
3. 个别只在桌面模式启用的治理型能力

## 五、当前真正的收尾重点

从 Web 化目标来看，接下来更应该做的是：

1. 删除或隔离桌面残留 API，减少误调用面
2. 清理旧兼容层，统一走已经完成的 Web API 主路径
3. 继续审查 Claude 旧集成等是否仍属于产品范围
4. 补文档，把“哪些功能是 Web 替代方案，不再提供桌面行为”写清楚

## 六、建议的后续执行顺序

1. 清理桌面专属入口
2. 清理旧兼容 API
3. 更新 README 与开发计划文档，明确 Web 版边界
4. 继续审查 `src-tauri` 中仅桌面有意义的命令与依赖，分批删除
