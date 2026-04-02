import { spawn } from "node:child_process";
import net from "node:net";
import process from "node:process";

const isWindows = process.platform === "win32";
const cargoCmd = isWindows ? "cargo.exe" : "cargo";

const children = [];
const defaultFrontendHost = "127.0.0.1";
const defaultFrontendPort = Number(process.env.CC_SWITCH_WEB_DEV_PORT || "3000");

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
    `[dev:web] no available frontend port found from ${preferredPort} to ${preferredPort + maxAttempts - 1}`,
  );
}

function startProcess(name, command, args, extraEnv = {}) {
  const env = {
    ...process.env,
    ...extraEnv,
  };

  const child = isWindows && command === "pnpm"
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
    console.error(`[${name}] failed to start`, error);
    shutdown(1);
  });

  child.on("exit", (code, signal) => {
    if (shuttingDown) {
      return;
    }

    if (signal) {
      console.log(`[${name}] exited with signal ${signal}`);
      return;
    }

    if ((code ?? 0) !== 0) {
      console.error(`[${name}] exited with code ${code}`);
      shutdown(code ?? 1);
    }
  });

  children.push(child);
  return child;
}

let shuttingDown = false;

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

async function main() {
  const apiBase = process.env.VITE_LOCAL_API_BASE || "http://127.0.0.1:8788";
  const frontendPort = await findAvailablePort(
    defaultFrontendHost,
    defaultFrontendPort,
  );

  console.log(`[dev:web] backend: ${apiBase}`);
  console.log(`[dev:web] frontend: http://${defaultFrontendHost}:${frontendPort}`);

  if (frontendPort !== defaultFrontendPort) {
    console.log(
      `[dev:web] port ${defaultFrontendPort} is in use, switched frontend to ${frontendPort}`,
    );
  }

  startProcess("web-service", cargoCmd, [
    "run",
    "--manifest-path",
    "backend/Cargo.toml",
    "--bin",
    "cc-switch-web",
  ]);

  startProcess(
    "web-ui",
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
    },
  );
}

main().catch((error) => {
  console.error("[dev:web] failed to start", error);
  process.exit(1);
});
