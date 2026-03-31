# CC Switch Web

[English](README.md) | 中文 | [日本語](README_JA.md)

## 项目说明

CC Switch Web 是 [cc-switch](https://github.com/farion1231/cc-switch) 的 Web 分支仓库。

当前仓库用于承载 CC Switch 的 Web 方向相关工作，包括 Web 端实现、相关实验以及分支上的定制化调整。

当前目标架构为：

- 前端：Web
- 后端：本地 Rust 服务
- 访问方式：浏览器访问 `http://localhost:xxxx`

这个方向除了常规的 Windows / Linux 本地环境，也重点面向无桌面的 Linux 服务器场景。

## 与上游项目的关系

- 上游项目：[cc-switch](https://github.com/farion1231/cc-switch)
- 当前仓库聚焦于 CC Switch 的 Web 分支方向
- 如果项目定位或对外描述发生变化，仓库内各语言版本 README 需要同步更新

## 说明

如果你要查看原始的 CC Switch 项目、桌面端应用或上游发布信息，请直接访问上游仓库。

## 运行方式

### 本地直接运行

1. 安装依赖：

   ```bash
   pnpm install --frozen-lockfile
   ```

2. 默认 Web 开发模式：

   ```bash
   pnpm dev
   ```

   等价于：

   ```bash
   pnpm dev:web
   ```

   然后打开 [http://localhost:3000](http://localhost:3000)。前端会连接本地 Rust 服务 `http://127.0.0.1:8788`。

3. 如果你想以前台方式运行 Docker 组合，也可以执行：

   ```bash
   pnpm dev:d
   ```

4. 以接近生产的方式本地运行：

   ```bash
   pnpm build:web
   pnpm start:web
   ```

   然后打开 [http://localhost:8788](http://localhost:8788)。

5. 构建一次后直接运行 release 二进制：

   ```bash
   pnpm build:web
   pnpm build:web:service
   ```

   Linux:

   ```bash
   pnpm start:l
   ```

   Windows:

   ```powershell
   pnpm start:w
   ```

### Docker 运行

1. 默认标准构建：

   ```bash
   pnpm build
   ```

   等价于：

   ```bash
   pnpm build:d
   ```

   这个命令会直接在 Docker 构建环境里完成前端和 Rust 服务构建。

2. 前台构建并启动：

   ```bash
   pnpm dev:d
   ```

   这个命令会以前台方式执行 `docker compose up --build`。

3. 如果镜像已经构建好，只想后台启动：

   ```bash
   pnpm up:d
   ```

4. 只重建镜像：

   ```bash
   pnpm build:d
   ```

5. 查看日志：

   ```bash
   pnpm logs:d
   ```

6. 停止：

   ```bash
   pnpm down:d
   ```

7. 打开 [http://localhost:8788](http://localhost:8788)。

8. 持久化数据默认保存在 `cc-switch-web-data` volume 中。

### Tauri 兼容命令

如果你只是临时保留桌面壳调试能力，可以显式使用：

```bash
pnpm dev:tauri
pnpm build:tauri
```

它们不再是当前仓库的默认推荐路径。

如果你希望容器内的服务直接管理宿主机上的 CLI 配置目录，需要在 `docker-compose.yml` 里额外挂载 `.claude`、`.codex`、`.gemini`、`opencode`、`openclaw` 等目录。
