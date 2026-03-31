# CC Switch Web 分发与启动体验方案

## 1. 目标

当前仓库只保留 Web 端形态，不再提供桌面 GUI 安装包。

统一目标为：

- 启动本地 Rust 服务
- 服务托管 `dist/` 前端静态资源
- 用户通过浏览器访问本地地址
- 各平台只在“启动方式”上有所区别，使用体验保持一致

## 2. 总体原则

- 不自动打开浏览器
- 启动后必须明确打印访问地址
- 优先提供“可解压直接运行”的分发包
- Linux 无桌面服务器必须是一级支持场景
- Docker 作为标准服务部署方式保留

## 3. 启动输出规范

所有本地启动脚本都应至少打印以下信息：

```text
CC Switch Web 已启动
监听地址: 127.0.0.1:8788
访问地址: http://127.0.0.1:8788
前端目录: ...
服务二进制: ...
按 Ctrl+C 停止服务
```

当监听地址为 `0.0.0.0` 时，额外打印：

```text
当前绑定到 0.0.0.0，请使用服务器 IP 或本机地址访问
```

## 4. 各平台预期体验

### 4.1 Windows

分发形态：

- `cc-switch-web-windows-x64.zip`

压缩包内容：

- `cc-switch-web.exe`
- `dist/`
- `run-web.cmd`
- `run-web.ps1`

用户体验：

- 解压后执行 `run-web.cmd`
- 本地启动 Rust 服务
- 终端打印访问地址
- 用户手动在浏览器打开打印出的 URL

### 4.2 Linux

分发形态：

- `cc-switch-web-linux-x64.tar.gz`

压缩包内容：

- `cc-switch-web`
- `dist/`
- `run-web.sh`

用户体验：

- 解压后执行 `bash run-web.sh`
- 本地或服务器环境启动 Rust 服务
- 终端打印访问地址
- 用户手动在浏览器访问，或配合 `systemd` 托管

### 4.3 macOS

分发形态：

- `cc-switch-web-macos-universal.zip`

压缩包内容：

- `cc-switch-web`
- `dist/`
- `run-web.command`
- `run-web.sh`

用户体验：

- 解压后执行 `run-web.command`
- 本地启动 Rust 服务
- 终端打印访问地址
- 用户手动在浏览器打开打印出的 URL

说明：

- macOS 分发包采用 `x86_64 + aarch64` 合并后的 universal 二进制
- 首次运行如遇到 Gatekeeper 拦截，按系统安全提示放行即可

### 4.4 Docker

分发形态：

- 仓库内 `Dockerfile`
- 仓库内 `docker-compose.yml`

用户体验：

- 使用 `docker compose up -d` 或 `docker run`
- 容器内启动本地 Rust 服务并托管前端静态资源
- 用户手动访问映射后的端口
- 不尝试做任何浏览器自动打开行为

## 5. Workflow 目标

需要保留两类 workflow：

### 5.1 Web CI

作用：

- `macOS / Windows / Linux` 跨平台静态校验
- Docker 构建与健康检查

### 5.2 Web Package

作用：

- 构建前端 `dist/`
- 构建平台二进制
- 产出平台分发包并上传为 GitHub Actions artifact

预期产物：

- `cc-switch-web-windows-x64.zip`
- `cc-switch-web-linux-x64.tar.gz`
- `cc-switch-web-macos-universal.zip`

## 6. 当前执行约束

- 仓库定位为 Web 分支，不再维护桌面 GUI 发布流
- 不再恢复旧 Tauri 安装包发布 workflow
- 如需后续接入镜像仓库发布，可在现有 Docker 基础上单独追加，不与桌面发布复用
