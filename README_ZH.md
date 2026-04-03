# CC Switch Web

[English](README.md) | 中文 | [日本語](README_JA.md)

## 项目说明

CC Switch Web 是 [cc-switch](https://github.com/farion1231/cc-switch) 的 Web 分支仓库。

当前仓库用于承载 CC Switch 的 Web 方向相关工作，包括 Web 端实现、相关实验以及分支上的定制化调整。

当前目标架构为：

- 前端：Web
- 后端：本地 Rust 服务
- 访问方式：浏览器访问 `http://localhost:xxxx`

这个方向面向 Windows、macOS、Linux 以及无桌面的 Linux 服务器场景。

## 当前版本

当前仓库版本为 `0.1.1`。

当前仓库现在以 `0.1.0` 作为 Web 分支的初始发布基线。此前继承的历史发布记录已从本仓库移除，如需查看更早历史，请以上游项目记录为准。

## 与上游项目的关系

- 上游项目：[cc-switch](https://github.com/farion1231/cc-switch)
- 当前 Web 仓库：[zuoliangyu/zuoliangyu-cc-switch-web](https://github.com/zuoliangyu/zuoliangyu-cc-switch-web)
- 作者：左岚（[哔哩哔哩](https://space.bilibili.com/27619688)）
- 当前仓库聚焦于 CC Switch 的 Web 分支方向
- 如果项目定位或对外描述发生变化，仓库内各语言版本 README 需要同步更新

## 说明

如果你要查看原始的 CC Switch 项目或上游发布信息，请直接访问上游仓库。

## 运行方式

### 命令速查

| 场景 | 命令 |
| --- | --- |
| 本地开发（`w`） | `pnpm dev` |
| Docker 前台开发（`d`） | `pnpm dev -- d` |
| 本地 release 构建（`w`） | `pnpm build` |
| Docker 镜像构建（`d`） | `pnpm build -- d` |
| 项目检查 | `pnpm check` |
| Windows 本地导出三类产物 | `.\scripts\package-artifacts.ps1` |

### 本地开发

1. 安装依赖：

   ```bash
   pnpm install --frozen-lockfile
   ```

   后端构建与检查需要 Rust `1.88+`。

2. 启动开发模式：

   ```bash
   pnpm dev
   ```

   显式写法：

   ```bash
   pnpm dev -- w
   ```

   Windows 下也可以直接执行：

   ```powershell
   .\scripts\dev.ps1 w
   ```

3. 打开 [http://localhost:3000](http://localhost:3000)。前端会连接本地 Rust 服务 `http://127.0.0.1:8788`。
   本地开发模式下请打开前端开发地址，不要直接打开 `8788`。`8788` 主要提供本地 API；为避免误用旧的 `dist` 静态资源，`pnpm dev` 现在会默认禁用后端静态前端托管。

4. `pnpm dev` 默认会打开本地调试请求日志：
   - 浏览器控制台会打印前端请求/响应日志
   - Rust 服务终端会打印 Web API 的 method/path/status/耗时
   - 如需手动覆盖，可设置 `VITE_RUNTIME_DEBUG_REQUESTS=0|1` 与 `CC_SWITCH_WEB_DEBUG_API=0|1`

### 本地 Release 二进制

1. 构建嵌入前端资源的 release 二进制：

   ```bash
   pnpm build
   ```

   显式写法：

   ```bash
   pnpm build -- w
   ```

   Windows 下也可以直接执行：

   ```powershell
   .\scripts\build.ps1 w
   ```

2. 输出路径：

   - Windows：`backend\target\release\cc-switch-web.exe`
   - Linux/macOS：`backend/target/release/cc-switch-web`

3. 直接运行对应二进制，然后打开 [http://localhost:8788](http://localhost:8788)。

4. 在本地 Web 服务模式下，CC Switch Web 自身的数据默认写入 CC Switch 使用的本地配置根目录：

   ```text
   ~/.cc-switch
   ```

   其中包括 `settings.json`、`cc-switch.db`、备份目录以及统一 Skills 存储等内容。旧的 `config.json` 不再属于当前 Web 运行时的主数据路径。

### Docker 运行

1. 构建 Docker 镜像：

   ```bash
   pnpm build -- d
   ```

   Windows 下也可以直接执行：

   ```powershell
   .\scripts\build.ps1 d
   ```

2. 以前台方式运行 Docker 组合：

   ```bash
   pnpm dev -- d
   ```

   Windows 下也可以直接执行：

   ```powershell
   .\scripts\dev.ps1 d
   ```

3. 如果镜像已经构建完成，想改为后台运行，请直接使用 Docker：

   ```bash
   docker compose up -d
   docker compose logs -f
   docker compose down
   ```

4. 打开 [http://localhost:8788](http://localhost:8788)。持久化数据默认保存在 `cc-switch-web-data` volume 中。

5. 如果你希望容器内服务直接管理宿主机上的 CLI 配置目录，先复制示例文件：

   ```bash
   cp docker-compose.host.example.yml docker-compose.host.yml
   ```

   然后按你的机器修改路径，再执行：

   ```bash
   docker compose -f docker-compose.yml -f docker-compose.host.yml up -d
   ```

   当前示例文件主要面向 Linux 服务器，默认使用 `$HOME` 下的 `.claude`、`.codex`、`.gemini`、`.config/opencode`、`.config/openclaw` 目录。

### Docker 内导出 Linux 包

如果你希望在不干扰宿主机环境的前提下导出 Linux 发布包，可以直接使用 Docker Buildx：

```bash
docker buildx build --target package-linux-tar --output type=local,dest=release/docker-linux .
```

导出压缩包：

```text
release/docker-linux/cc-switch-web-linux-x64.tar.gz
```

如果你想直接导出未压缩目录：

```bash
docker buildx build --target package-linux-dir --output type=local,dest=release/docker-linux .
```

导出目录：

```text
release/docker-linux/cc-switch-web-linux-x64/
```

目录内只包含单文件可执行程序 `cc-switch-web`，解压后直接运行即可。

当前导出的 Linux 二进制为 `x86_64-unknown-linux-musl` 静态链接版本，可尽量减少宿主机运行库差异导致的问题。

### Windows 本地导出产物

如果你当前在 Windows，并且本机已经安装好 Rust 与 Docker / Buildx，可以直接执行：

```powershell
.\scripts\package-artifacts.ps1
```

这个脚本会一次生成三类产物：

- Windows 可执行文件：`release\local-artifacts\windows\cc-switch-web.exe`
- Linux 发布包：`release\local-artifacts\linux\cc-switch-web-linux-x64.tar.gz`
- Docker 镜像包：`release\local-artifacts\docker\cc-switch-web-docker-image.tar.gz`

其中：

- Windows 产物来自本机 `cargo build --locked --release`
- Linux 产物来自 Docker Buildx 的 `package-linux-tar` stage
- Docker 镜像包可通过下面命令导入：

```powershell
docker load -i .\release\local-artifacts\docker\cc-switch-web-docker-image.tar.gz
```

### Linux systemd 示例

如果你要在无桌面的 Linux 服务器上长期托管服务，可以使用仓库中的示例文件：

`deploy/systemd/cc-switch-web.service.example`

推荐步骤：

1. 在 Linux 上执行 `pnpm build` 生成二进制，或者把已打包好的 Linux 二进制放到 `/opt/cc-switch-web`。

2. 复制服务文件到系统目录：

   ```bash
   sudo cp deploy/systemd/cc-switch-web.service.example /etc/systemd/system/cc-switch-web.service
   ```

3. 按你的机器修改下面这些字段：
   - `User`
   - `Group`
   - `WorkingDirectory`
   - `HOME`
   - `ExecStart`

4. 重新加载并启动：

   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now cc-switch-web
   ```

5. 查看状态和日志：

   ```bash
   sudo systemctl status cc-switch-web
   sudo journalctl -u cc-switch-web -f
   ```
