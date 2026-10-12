// Starts tauri-driver (WebDriver bridge to the desktop app) for the E2E run.
import { spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";

let driver: ChildProcess | undefined;
let dataDir: string | undefined;

function waitForPort(port: number, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve, reject) => {
    const attempt = () => {
      const socket = net.connect(port, "127.0.0.1", () => {
        socket.end();
        resolve();
      });
      socket.on("error", () => {
        socket.destroy();
        if (Date.now() > deadline) reject(new Error(`tauri-driver did not start on port ${port}`));
        else setTimeout(attempt, 200);
      });
    };
    attempt();
  });
}

export async function setup() {
  // Fresh database for every run; the app reads it from this variable.
  dataDir = mkdtempSync(path.join(os.tmpdir(), "rodi-e2e-"));
  process.env.RODI_CHECKER_DATA_DIR = dataDir;
  const binary = path.join(os.homedir(), ".cargo", "bin", process.platform === "win32" ? "tauri-driver.exe" : "tauri-driver");
  driver = spawn(process.env.TAURI_DRIVER ?? binary, [], { stdio: "inherit", env: process.env });
  await waitForPort(4444, 20_000);
}

export async function teardown() {
  driver?.kill();
  if (dataDir) rmSync(dataDir, { recursive: true, force: true });
}
