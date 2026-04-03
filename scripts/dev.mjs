import { spawn } from "node:child_process";
import net from "node:net";
import process from "node:process";

const mode = (process.argv[2] || "w").toLowerCase();
const isWindows = process.platform === "win32";
const cargoCmd = isWindows ? "cargo.exe" : "cargo";
const defaultFrontendHost = "127.0.0.1";
const defaultFrontendPort = Number(process.env.CC_SWITCH_WEB_DEV_PORT || "3000");
const children = [];
let shuttingDown = false;

function printUsage() {
  console.log("Usage: pnpm dev -- <w|d>");
  console.log("  w: local development mode");
  console.log("  d: Docker foreground development mode");
}

function spawnCommand(command, args, extraEnv = {}) {
  const env = {
    ...process.env,
    ...extraEnv,
  };

  const child =
    isWindows && command === "pnpm"
      ? spawn("cmd.exe", ["/d", "/s", "/c", command, ...args], {
          cwd: process.cwd(),
          stdio: "inherit",
          env,
        })
      : spawn(command, args, {
          cwd: process.cwd(),
          stdio: "inherit",
          env,
        });

  child.on("error", (error) => {
    console.error(`[${command}] failed to start`, error);
    shutdown(1);
  });

  child.on("exit", (code, signal) => {
    if (shuttingDown) {
      return;
    }

    if (signal) {
      console.log(`[${command}] exited with signal ${signal}`);
      return;
    }

    if ((code ?? 0) !== 0) {
      console.error(`[${command}] exited with code ${code}`);
      shutdown(code ?? 1);
    }
  });

  children.push(child);
  return child;
}

function canListen(host, port) {
  return new Promise((resolve) => {
    const server = net.createServer();

    server.once("error", () => {
      resolve(false);
    });

    server.once("listening", () => {
      server.close(() => resolve(true));
    });

    server.listen(port, host);
  });
}

async function findAvailablePort(host, preferredPort, maxAttempts = 20) {
  for (let offset = 0; offset < maxAttempts; offset += 1) {
    const port = preferredPort + offset;
    // eslint-disable-next-line no-await-in-loop
    const available = await canListen(host, port);
    if (available) {
      return port;
    }
  }

  throw new Error(
    `[dev] no available frontend port found from ${preferredPort} to ${preferredPort + maxAttempts - 1}`,
  );
}

function shutdown(exitCode = 0) {
  if (shuttingDown) {
    return;
  }
  shuttingDown = true;

  for (const child of children) {
    if (!child.killed) {
      child.kill("SIGTERM");
    }
  }

  setTimeout(() => {
    for (const child of children) {
      if (!child.killed) {
        child.kill("SIGKILL");
      }
    }
    process.exit(exitCode);
  }, 1500).unref();
}

process.on("SIGINT", () => shutdown(0));
process.on("SIGTERM", () => shutdown(0));

async function runLocalDevelopment() {
  const apiBase = process.env.VITE_LOCAL_API_BASE || "http://127.0.0.1:8788";
  const frontendPort = await findAvailablePort(
    defaultFrontendHost,
    defaultFrontendPort,
  );

  console.log("[dev] mode=w -> local development mode");
  console.log(`[dev] backend: ${apiBase}`);
  console.log(`[dev] frontend: http://${defaultFrontendHost}:${frontendPort}`);
  console.log("[dev] request debug logs: enabled");
  console.log("[dev] backend static frontend: disabled");
  console.log(
    `[dev] open the app in browser at http://${defaultFrontendHost}:${frontendPort}, not ${apiBase}`,
  );

  if (frontendPort !== defaultFrontendPort) {
    console.log(
      `[dev] port ${defaultFrontendPort} is in use, switched frontend to ${frontendPort}`,
    );
  }

  spawnCommand(cargoCmd, [
    "run",
    "--manifest-path",
    "backend/Cargo.toml",
    "--bin",
    "cc-switch-web",
  ], {
    CC_SWITCH_WEB_DEBUG_API:
      process.env.CC_SWITCH_WEB_DEBUG_API || "1",
    CC_SWITCH_WEB_DISABLE_STATIC:
      process.env.CC_SWITCH_WEB_DISABLE_STATIC || "1",
    RUST_LOG: process.env.RUST_LOG || "info",
  });

  spawnCommand(
    "pnpm",
    [
      "exec",
      "vite",
      "--host",
      defaultFrontendHost,
      "--port",
      String(frontendPort),
    ],
    {
      VITE_LOCAL_API_BASE: apiBase,
      VITE_RUNTIME_DEBUG_REQUESTS:
        process.env.VITE_RUNTIME_DEBUG_REQUESTS || "1",
    },
  );
}

async function main() {
  switch (mode) {
    case "w":
      await runLocalDevelopment();
      break;
    case "d":
      console.log("[dev] mode=d -> Docker foreground development mode");
      spawnCommand("docker", ["compose", "up", "--build"]);
      break;
    default:
      console.error(`[dev] unsupported argument: ${mode}`);
      printUsage();
      process.exit(1);
  }
}

main().catch((error) => {
  console.error("[dev] failed to start", error);
  process.exit(1);
});
