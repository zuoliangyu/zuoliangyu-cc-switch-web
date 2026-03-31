# Web 化剩余缺口审计

更新时间：2026-04-01

## 结论

当前仓库的 Web 主业务能力已经基本拉齐。

补充说明：

- 本地运行的 Web 版不是“只存在浏览器里的临时前端”
- 本地 Rust 服务默认仍然使用 `~/.cc-switch` 作为 CC Switch 自身数据目录
- 也就是说，本地 Web 模式下的 `config.json`、`settings.json`、`cc-switch.db`、备份、Skills SSOT 等默认与原本地端保持同一路径策略
- Docker 运行是例外：默认写入容器 volume，而不是宿主机 `~/.cc-switch`

剩余未迁移项主要分为两类：

1. 仍建议继续处理，但以“收尾/重构/兼容收口”为主，而不是核心业务缺失
2. 明确属于桌面端/Tauri 壳能力，建议直接删除、隐藏或保留为仅 Tauri 生效，不再做 Web 映射

换句话说：

- `Providers / Proxy / Failover / Settings / MCP / Prompts / Skills / Workspace / Sessions / Usage / OpenClaw / Universal Provider` 的 Web 主路径已基本可用
- 当前更大的问题已经不是“缺少 Web 核心功能”，而是“仓库里还残留不少桌面 API 和旧兼容层”

## 一、建议继续处理

| 项目 | 当前状态 | 建议动作 | 原因 |
|---|---|---|---|
| `sync_current_providers_live` | 仍有工具层调用 | 评估后保留或改成显式 Web API | 这不是主页面独立入口，但在导入/变更后同步 live 状态的兼容链路里仍可能有价值 |
| `set_common_config_snippet` / `set_global_proxy_url` / 部分旧设置命令 | 存在旧调用痕迹 | 逐项核对是否还有业务价值 | 有些能力可能已被新版 Settings/Proxy 页面替代，需要确认后再删 |
| OMO / OMO Slim 相关命令 | 仍有残留 API | 先核对是否还属于产品范围 | 如果还要保留对应能力，需要决定是迁移到 Web，还是整体下线 |
| 旧兼容 Skills API | 工具层仍保留 | 逐步移除对旧别名的依赖 | 当前实际主路径已走统一 Skills API，不宜长期双轨 |
| 旧兼容 MCP API | 工具层仍保留 | 收敛到统一 MCP API | 当前 Web 主路径已完成统一 MCP 管理，这批兼容命令应减量 |

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
| `open_provider_terminal` | Web 未映射 | 不再迁移 | Web 页面已隐藏该入口 |
| `launch_session_terminal` | Web 未映射 | 不再迁移 | Web 页面已降级为复制恢复命令 |
| `open_app_config_folder` / `open_config_folder` | Web 未映射 | 不再迁移 | 与本地 GUI 文件管理器强耦合 |
| `apply_claude_plugin_config` | Web 未映射 | 建议下线或桌面专用 | 属于旧桌面集成逻辑 |
| `apply_claude_onboarding_skip` / `clear_claude_onboarding_skip` | Web 未映射 | 建议下线或桌面专用 | 属于 Claude 桌面集成兼容逻辑 |
| Deeplink 相关命令 | Web 未映射 | 建议删除或冻结 | Web 本地服务模式下优先级极低 |
| 单实例/窗口/托盘相关后续残留 | 未纳入 Web | 建议继续清理 | 与目标架构不一致 |

## 三、已降级处理，不应再算作“Web 功能缺失”

这些能力在 Web 下不是“没做”，而是已经做了有意识的替代方案：

| 原桌面能力 | Web 当前方案 |
|---|---|
| 打开工作区目录 | 复制目录路径 |
| 打开 Session 终端 | 复制恢复命令 |
| 打开 Provider 专属终端 | Web 下隐藏入口 |
| 原生文件导入 | 浏览器上传 SQL |
| 原生文件导出 | 浏览器下载 SQL |
| 原生目录选择器 | 手动填写目录覆盖 |
| 桌面自动更新 | Web 下隐藏 |

## 四、当前真正的收尾重点

从 Web 化目标来看，接下来更应该做的是：

1. 删除或隔离桌面残留 API，减少误调用面
2. 清理旧兼容层，统一走已经完成的 Web API 主路径
3. 审查 OMO / OMO Slim、Claude 旧集成、Deeplink 是否还属于产品范围
4. 补文档，把“哪些功能是 Web 替代方案，不再提供桌面行为”写清楚

## 五、建议的后续执行顺序

1. 清理桌面专属入口
2. 清理旧兼容 API
3. 评估并处理 OMO / OMO Slim 残留
4. 更新 README 与开发计划文档，明确 Web 版边界
