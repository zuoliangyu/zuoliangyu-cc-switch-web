# CC Switch Web

English | [中文](README_ZH.md) | [日本語](README_JA.md)

## Overview

CC Switch Web is the web branch repository of [cc-switch](https://github.com/farion1231/cc-switch).

This repository is used to carry web-oriented work around CC Switch, including web-side implementation, related experiments, and branch-specific adjustments.

The current target architecture is:

- Frontend: Web
- Backend: local Rust service
- Access pattern: browser opens `http://localhost:xxxx`

This direction is intended to support headless Linux servers in addition to regular Windows and Linux desktop environments.

## Relationship to Upstream

- Upstream project: [cc-switch](https://github.com/farion1231/cc-switch)
- This repository focuses on the Web branch direction of CC Switch
- When project positioning or external description changes, all language README files in this repository should be updated together

## Notes

If you are looking for the original CC Switch project, desktop application, or upstream release information, please visit the upstream repository directly.

## Run

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

5. Build once and run the release binary directly:

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

### Tauri Compatibility

If you still need the desktop shell temporarily for debugging, use:

```bash
pnpm dev:tauri
pnpm build:tauri
```

These are no longer the default path for this repository.

If you want the containerized service to manage host-side CLI configuration directories directly, add bind mounts in `docker-compose.yml` for paths such as `.claude`, `.codex`, `.gemini`, `opencode`, and `openclaw`.
