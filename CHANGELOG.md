# Changelog

本仓库从 Web 分支独立维护开始，重新以 `0.1.0` 作为初始版本。

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
