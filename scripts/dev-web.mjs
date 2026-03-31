import { spawn } from "node:child_process";
import process from "node:process";

const isWindows = process.platform === "win32";
const pnpmCmd = isWindows ? "pnpm.cmd" : "pnpm";
const cargoCmd = isWindows ? "cargo.exe" : "cargo";

const children = [];

function startProcess(name, command, args, extraEnv = {}) {
  const child = spawn(command, args, {
    cwd: process.cwd(),
    stdio: "inherit",
    env: {
      ...process.env,
      ...extraEnv,
    },
  });

  child.on("exit", (code, signal) => {
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

const apiBase = process.env.VITE_LOCAL_API_BASE || "http://127.0.0.1:8788";

console.log(`[dev:web] backend: ${apiBase}`);
console.log("[dev:web] frontend: http://127.0.0.1:3000");

startProcess("web-service", cargoCmd, [
  "run",
  "--manifest-path",
  "src-tauri/Cargo.toml",
  "--bin",
  "cc-switch-web",
]);

startProcess(
  "web-ui",
  pnpmCmd,
  ["exec", "vite", "--host", "127.0.0.1", "--port", "3000"],
  {
    VITE_LOCAL_API_BASE: apiBase,
  },
);
