import { spawnSync } from "node:child_process";
import process from "node:process";

const isWindows = process.platform === "win32";
const cargoCmd = isWindows ? "cargo.exe" : "cargo";

function run(command, args) {
  const result =
    isWindows && command === "pnpm"
      ? spawnSync("cmd.exe", ["/d", "/s", "/c", command, ...args], {
          cwd: process.cwd(),
          stdio: "inherit",
          env: process.env,
        })
      : spawnSync(command, args, {
          cwd: process.cwd(),
          stdio: "inherit",
          env: process.env,
        });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

console.log("[check] validating Node scripts");
run(process.execPath, ["--check", "scripts/dev.mjs"]);
run(process.execPath, ["--check", "scripts/build.mjs"]);
run(process.execPath, ["--check", "scripts/check.mjs"]);

console.log("[check] running TypeScript check");
run("pnpm", ["exec", "tsc", "--noEmit", "-p", "tsconfig.json"]);

console.log("[check] running Rust check");
run(cargoCmd, [
  "check",
  "--locked",
  "--manifest-path",
  "backend/Cargo.toml",
  "--bin",
  "cc-switch-web",
]);
