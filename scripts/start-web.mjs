import { existsSync } from "node:fs";
import { spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";

const isWindows = process.platform === "win32";
const cargoCmd = isWindows ? "cargo.exe" : "cargo";
const distDir =
  process.env.CC_SWITCH_WEB_DIST_DIR || path.join(process.cwd(), "dist");
const host = process.env.CC_SWITCH_WEB_HOST || "127.0.0.1";
const port = process.env.CC_SWITCH_WEB_PORT || "8788";

if (!existsSync(distDir)) {
  console.error(`[start:web] dist 目录不存在: ${distDir}`);
  console.error("[start:web] 请先执行: pnpm build:web");
  process.exit(1);
}

console.log("CC Switch Web 已启动");
console.log(`监听地址: ${host}:${port}`);
console.log(`访问地址: http://${host}:${port}`);
console.log(`前端目录: ${distDir}`);
console.log(
  "服务命令: cargo run --manifest-path src-tauri/Cargo.toml --bin cc-switch-web",
);
if (host === "0.0.0.0") {
  console.log("当前绑定到 0.0.0.0，请使用服务器 IP 或本机地址访问");
}
console.log("按 Ctrl+C 停止服务");
console.log("");

const child = spawn(
  cargoCmd,
  ["run", "--manifest-path", "src-tauri/Cargo.toml", "--bin", "cc-switch-web"],
  {
    cwd: process.cwd(),
    stdio: "inherit",
    env: {
      ...process.env,
      CC_SWITCH_WEB_HOST: host,
      CC_SWITCH_WEB_PORT: port,
      CC_SWITCH_WEB_DIST_DIR: distDir,
    },
  },
);

child.on("exit", (code, signal) => {
  if (signal) {
    process.exit(0);
  }
  process.exit(code ?? 0);
});
