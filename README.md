# CC Switch Web

English | [中文](README_ZH.md) | [日本語](README_JA.md)

## Overview

CC Switch Web is the web branch repository of [cc-switch](https://github.com/farion1231/cc-switch).

This repository is used to carry web-oriented work around CC Switch, including web-side implementation, related experiments, and branch-specific adjustments.

The current target architecture is:

- Frontend: Web
- Backend: local Rust service
- Access pattern: browser opens `http://localhost:xxxx`

This direction targets Windows, macOS, Linux, and headless Linux server environments.

## Version

The current repository version is `0.1.1`.

This repository now treats `0.1.0` as its initial Web release baseline. Previous inherited release history has been removed from this repository and should be considered part of the upstream project history.

## Relationship to Upstream

- Upstream project: [cc-switch](https://github.com/farion1231/cc-switch)
- Current Web repository: [zuoliangyu/zuoliangyu-cc-switch-web](https://github.com/zuoliangyu/zuoliangyu-cc-switch-web)
- Author: 左岚 ([Bilibili](https://space.bilibili.com/27619688))
- This repository focuses on the Web branch direction of CC Switch
- When project positioning or external description changes, all language README files in this repository should be updated together

## Notes

If you are looking for the original CC Switch project or upstream release information, please visit the upstream repository directly.

## Run

### Quick Commands

| Scenario | Command |
| --- | --- |
| Local development (`w`) | `pnpm dev` |
| Docker foreground development (`d`) | `pnpm dev -- d` |
| Local release build (`w`) | `pnpm build` |
| Docker image build (`d`) | `pnpm build -- d` |
| Project check | `pnpm check` |
| Export Windows/Linux/Docker artifacts on Windows | `.\scripts\package-artifacts.ps1` |

### Local Development

1. Install dependencies:

   ```bash
   pnpm install --frozen-lockfile
   ```

   Rust `1.88+` is required for the backend build and check steps.

2. Start development mode:

   ```bash
   pnpm dev
   ```

   Equivalent explicit form:

   ```bash
   pnpm dev -- w
   ```

   On Windows, you can also run:

   ```powershell
   .\scripts\dev.ps1 w
   ```

3. Open [http://localhost:3000](http://localhost:3000). The frontend connects to the local Rust service at `http://127.0.0.1:8788`.
   In local development, open the frontend dev URL instead of `8788`. Port `8788` is primarily for the local API, and `pnpm dev` now disables backend static frontend hosting by default to avoid accidentally serving an old `dist` bundle.

4. `pnpm dev` enables local request debug logs by default:
   - Browser DevTools show frontend request/response logs
   - The Rust service terminal shows Web API method/path/status/duration logs
   - You can override this with `VITE_RUNTIME_DEBUG_REQUESTS=0|1` and `CC_SWITCH_WEB_DEBUG_API=0|1`

### Local Release Binary

1. Build the embedded release binary:

   ```bash
   pnpm build
   ```

   Equivalent explicit form:

   ```bash
   pnpm build -- w
   ```

   On Windows, you can also run:

   ```powershell
   .\scripts\build.ps1 w
   ```

2. Output path:

   - Windows: `backend\target\release\cc-switch-web.exe`
   - Linux/macOS: `backend/target/release/cc-switch-web`

3. Run the binary directly, then open [http://localhost:8788](http://localhost:8788).

4. In local Web service mode, CC Switch Web stores its own data under the default CC Switch local config root:

   ```text
   ~/.cc-switch
   ```

   This includes files such as `settings.json`, `cc-switch.db`, backup data, and the unified Skills storage. Legacy `config.json` is not part of the active Web runtime data path.

### Docker

1. Build the Docker image:

   ```bash
   pnpm build -- d
   ```

   On Windows, you can also run:

   ```powershell
   .\scripts\build.ps1 d
   ```

2. Run the Docker stack in the foreground:

   ```bash
   pnpm dev -- d
   ```

   On Windows, you can also run:

   ```powershell
   .\scripts\dev.ps1 d
   ```

3. If you want background mode after the image is built, use Docker directly:

   ```bash
   docker compose up -d
   docker compose logs -f
   docker compose down
   ```

4. Open [http://localhost:8788](http://localhost:8788). Persistent data is stored in the `cc-switch-web-data` volume.

5. If you want the containerized service to manage host-side CLI config directories directly, first copy the example file:

   ```bash
   cp docker-compose.host.example.yml docker-compose.host.yml
   ```

   Then adjust the paths for your machine and run:

   ```bash
   docker compose -f docker-compose.yml -f docker-compose.host.yml up -d
   ```

   The example file is primarily for Linux servers and uses `$HOME` paths for `.claude`, `.codex`, `.gemini`, `.config/opencode`, and `.config/openclaw`.

### Export Linux Package Inside Docker

If you want a Linux release package without polluting the host build environment, use Docker Buildx directly:

```bash
docker buildx build --target package-linux-tar --output type=local,dest=release/docker-linux .
```

Exported archive:

```text
release/docker-linux/cc-switch-web-linux-x64.tar.gz
```

If you want the unpacked directory instead:

```bash
docker buildx build --target package-linux-dir --output type=local,dest=release/docker-linux .
```

Exported directory:

```text
release/docker-linux/cc-switch-web-linux-x64/
```

The package contains the single executable `cc-switch-web`. After extracting on Linux, run that binary directly.

The exported Linux binary is built as `x86_64-unknown-linux-musl`, which reduces host-side runtime dependency issues.

### Export Artifacts On Windows

If you are working on Windows and already have Rust plus Docker/Buildx installed locally, run:

```powershell
.\scripts\package-artifacts.ps1
```

The script generates all three local artifacts in one pass:

- Windows executable: `release\local-artifacts\windows\cc-switch-web.exe`
- Linux release package: `release\local-artifacts\linux\cc-switch-web-linux-x64.tar.gz`
- Docker image archive: `release\local-artifacts\docker\cc-switch-web-docker-image.tar.gz`

Details:

- The Windows artifact comes from local `cargo build --locked --release`
- The Linux artifact comes from Docker Buildx using the `package-linux-tar` stage
- The Docker image archive can be imported with:

```powershell
docker load -i .\release\local-artifacts\docker\cc-switch-web-docker-image.tar.gz
```

### Linux systemd Example

If you want to keep the service running on a headless Linux server, use:

`deploy/systemd/cc-switch-web.service.example`

Recommended steps:

1. Build the release binary on Linux, or copy a packaged Linux artifact into `/opt/cc-switch-web`.

2. Copy the service file into the system directory:

   ```bash
   sudo cp deploy/systemd/cc-switch-web.service.example /etc/systemd/system/cc-switch-web.service
   ```

3. Adjust these fields for your machine:
   - `User`
   - `Group`
   - `WorkingDirectory`
   - `HOME`
   - `ExecStart`

4. Reload and start:

   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now cc-switch-web
   ```

5. Check status and logs:

   ```bash
   sudo systemctl status cc-switch-web
   sudo journalctl -u cc-switch-web -f
   ```
