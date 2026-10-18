import { type ChildProcess, spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";

const application = path.resolve(import.meta.dirname, "../../../target/debug/voxel");
const tauriDriverBin = process.env.TAURI_DRIVER ?? "tauri-driver";

let tauriDriver: ChildProcess | undefined;
let closing = false;

function closeTauriDriver() {
  closing = true;
  tauriDriver?.kill();
}

export const config: WebdriverIO.Config = {
  runner: "local",
  hostname: "127.0.0.1",
  port: 4444,
  specs: ["./specs/**/*.e2e.ts"],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      "tauri:options": { application },
    } as WebdriverIO.Capabilities,
  ],
  logLevel: "warn",
  reporters: ["spec"],
  framework: "mocha",
  mochaOpts: {
    ui: "bdd",
    timeout: 60_000,
  },

  beforeSession() {
    tauriDriver = spawn(tauriDriverBin, [], { stdio: [null, process.stdout, process.stderr] });
    tauriDriver.on("error", (error) => {
      console.error("Falha ao iniciar o tauri-driver:", error);
      process.exit(1);
    });
    tauriDriver.on("exit", (code) => {
      if (!closing) {
        console.error("tauri-driver encerrou com código:", code);
        process.exit(1);
      }
    });
  },

  afterSession() {
    closeTauriDriver();
  },
};

for (const signal of ["exit", "SIGINT", "SIGTERM", "SIGHUP"] as const) {
  process.on(signal, closeTauriDriver);
}
