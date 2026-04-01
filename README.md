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

## Relationship to Upstream

- Upstream project: [cc-switch](https://github.com/farion1231/cc-switch)
- This repository focuses on the Web branch direction of CC Switch
- When project positioning or external description changes, all language README files in this repository should be updated together

## Notes

If you are looking for the original CC Switch project or upstream release information, please visit the upstream repository directly.

## Run

### Quick Commands

| Scenario                | Command        |
| ----------------------- | -------------- |
| Default Web development | `pnpm dev`     |
| Foreground Docker stack | `pnpm dev:d`   |
| Standard Docker build   | `pnpm build`   |
| Background Docker start | `pnpm up:d`    |
| Follow Docker logs      | `pnpm logs:d`  |
| Stop Docker stack       | `pnpm down:d`  |
| Package Linux via Docker | `pnpm build:pkg:l` |
| Direct run on macOS     | `pnpm start:m` |
| Direct run on Linux     | `pnpm start:l` |
| Direct run on Windows   | `pnpm start:w` |

### Local Run

1. Install dependencies:

   ```bash
   pnpm install --frozen-lockfile
   ```

2. Default Web development mode:

   ```bash
   pnpm dev
   ```

   Equivalent to:

   ```bash
   pnpm dev:web
   ```

   Open [http://localhost:3000](http://localhost:3000). The frontend talks to the local Rust service at `http://127.0.0.1:8788`.

3. If you want to run the Docker stack in the foreground, use:

   ```bash
   pnpm dev:d
   ```

4. Start a production-style local run:

   ```bash
   pnpm build:web
   pnpm start:web
   ```

   Then open [http://localhost:8788](http://localhost:8788).

   In local Web service mode, CC Switch Web stores its own data under the default local config root used by CC Switch:

   ```text
   ~/.cc-switch
   ```

   This includes files such as `config.json`, `settings.json`, `cc-switch.db`, backups, and the unified Skills storage.

5. Build once and run the release binary directly:

   ```bash
   pnpm build:web
   pnpm build:web:service
   ```

   Linux:

   ```bash
   pnpm start:l
   ```

   macOS:

   ```bash
   pnpm start:m
   ```

   Windows:

   ```powershell
   pnpm start:w
   ```

   The launcher scripts only start the local service and print the access URL. They do not open the browser automatically.

### Docker Run

1. Default standard build:

   ```bash
   pnpm build
   ```

   Equivalent to:

   ```bash
   pnpm build:d
   ```

   This builds the frontend and Rust service directly inside the Docker build environment.

2. Build and start in the foreground:

   ```bash
   pnpm dev:d
   ```

   This runs `docker compose up --build` in the foreground.

3. If the image is already built and you only want to start it in the background:

   ```bash
   pnpm up:d
   ```

4. Rebuild image only:

   ```bash
   pnpm build:d
   ```

5. View logs:

   ```bash
   pnpm logs:d
   ```

6. Stop:

   ```bash
   pnpm down:d
   ```

7. Open [http://localhost:8788](http://localhost:8788).

8. Persistent data is stored in the `cc-switch-web-data` volume.

9. If you want the containerized service to manage host-side CLI config directories directly, first copy the example file:

   ```bash
   cp docker-compose.host.example.yml docker-compose.host.yml
   ```

   Then adjust the paths for your machine and run:

   ```bash
   docker compose -f docker-compose.yml -f docker-compose.host.yml up -d
   ```

   The example file is primarily for Linux servers and uses `$HOME` paths for `.claude`, `.codex`, `.gemini`, `.config/opencode`, and `.config/openclaw`.

### Export Linux Package Inside Docker

If you want a Linux release package without polluting the host build environment, run:

```bash
pnpm build:pkg:l
```

This uses `docker buildx build` and exports:

```text
release/docker-linux/cc-switch-web-linux-x64.tar.gz
```

If you want the unpacked directory instead, run:

```bash
pnpm build:pkg:l:dir
```

The exported directory is:

```text
release/docker-linux/cc-switch-web-linux-x64/
```

The package contains:

- `cc-switch-web`
- `dist/`
- `run-web.sh`

After extracting on Linux, run:

```bash
bash run-web.sh
```

### Linux systemd Example

If you want to keep the service running on a headless Linux server, use the example file in the repository:

`deploy/systemd/cc-switch-web.service.example`

Recommended steps:

1. Build the frontend and local service first:

   ```bash
   pnpm build:web
   pnpm build:web:service
   ```

2. Copy the service file into the system directory:

   ```bash
   sudo cp deploy/systemd/cc-switch-web.service.example /etc/systemd/system/cc-switch-web.service
   ```

3. Adjust these fields for your machine:
   - `User`
   - `Group`
   - `WorkingDirectory`
   - `HOME`
   - `CC_SWITCH_WEB_DIST_DIR`

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
