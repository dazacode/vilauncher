#!/usr/bin/env node
// Runs the same checks as .github/workflows/build.yml locally and prints a
// compact report: one PASS/FAIL line per step, and the last lines of output
// for any step that failed. ANSI colour codes are stripped.
//
// Usage:
//   node scripts/ci.mjs                      run install, typecheck, i18n, vite, test
//   node scripts/ci.mjs --only typecheck,test
//   node scripts/ci.mjs --tauri              also run the installer build
//   node scripts/ci.mjs --keep-going         run every step even after a failure
//   node scripts/ci.mjs --json               also write .ci/last-run.json
//
// Exit code is 0 only when every step that ran passed.

import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const TAIL_LINES = 40;
const ANSI = /\u001b\[[0-?]*[ -/]*[@-~]|\u001b\][^\u0007]*(\u0007|\u001b\\)/g;

// Order matters: this is the order CI runs them in.
const STEPS = [
  { id: "install", name: "install", cmd: "pnpm install --frozen-lockfile", cwd: ROOT, optional: false },
  { id: "typecheck", name: "typecheck", cmd: "pnpm exec tsc -b", cwd: ROOT, optional: false },
  { id: "i18n", name: "translations", cmd: "pnpm i18n:check", cwd: ROOT, optional: false },
  { id: "vite", name: "vite build", cmd: "pnpm exec vite build", cwd: ROOT, optional: false },
  { id: "test", name: "cargo test", cmd: "cargo test --locked", cwd: join(ROOT, "src-tauri"), optional: false },
  { id: "tauri", name: "tauri build", cmd: "pnpm tauri build", cwd: ROOT, optional: true },
];
const STEP_IDS = STEPS.map((s) => s.id);

function usage(exitCode) {
  const text = `Usage: node scripts/ci.mjs [--only <ids>] [--tauri] [--skip-tauri] [--keep-going] [--json]
Step ids: ${STEP_IDS.join(", ")}
  --only <ids>   comma-separated subset to run (repeatable)
  --tauri        include the installer build (tauri), off by default
  --skip-tauri   keep the installer build off (default; kept for clarity)
  --keep-going   run all steps even after a failure
  --json         write a machine-readable summary to .ci/last-run.json`;
  (exitCode === 0 ? console.log : console.error)(text);
  process.exit(exitCode);
}

function parseArgs(argv) {
  const opts = { only: null, tauri: false, keepGoing: false, json: false };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") usage(0);
    else if (arg === "--only") {
      const value = argv[++i];
      if (!value) usage(2);
      opts.only = [...(opts.only ?? []), ...value.split(",").map((s) => s.trim()).filter(Boolean)];
    } else if (arg.startsWith("--only=")) {
      opts.only = [...(opts.only ?? []), ...arg.slice(7).split(",").map((s) => s.trim()).filter(Boolean)];
    } else if (arg === "--tauri") opts.tauri = true;
    else if (arg === "--skip-tauri") opts.tauri = false;
    else if (arg === "--keep-going") opts.keepGoing = true;
    else if (arg === "--json") opts.json = true;
    else {
      console.error(`Unknown argument: ${arg}`);
      usage(2);
    }
  }
  if (opts.only) {
    const unknown = opts.only.filter((id) => !STEP_IDS.includes(id));
    if (unknown.length) {
      console.error(`Unknown step id(s): ${unknown.join(", ")}. Valid: ${STEP_IDS.join(", ")}`);
      process.exit(2);
    }
  }
  return opts;
}

function selectSteps(opts) {
  return STEPS.filter((step) => {
    if (opts.only) return opts.only.includes(step.id);
    if (step.optional) return opts.tauri;
    return true;
  });
}

function stripAnsi(text) {
  return text.replace(ANSI, "").replace(/\r/g, "");
}

// Runs one command, returns { exitCode, signal, durationMs, lines, error }.
// Output is captured (not streamed) so the report stays compact.
function runStep(step) {
  return new Promise((resolve) => {
    const started = Date.now();
    const lines = [];
    let partial = "";
    const onData = (chunk) => {
      partial += stripAnsi(chunk.toString("utf8"));
      const parts = partial.split("\n");
      partial = parts.pop();
      lines.push(...parts);
    };

    let child;
    try {
      child = spawn(step.cmd, {
        cwd: step.cwd,
        // shell: true lets Windows resolve pnpm.cmd and cargo via cmd.exe.
        // The command strings are fixed above, so no user input reaches the shell.
        shell: true,
        windowsHide: true,
        env: { ...process.env, CARGO_TERM_COLOR: "never", NO_COLOR: "1", FORCE_COLOR: "0" },
        stdio: ["ignore", "pipe", "pipe"],
      });
    } catch (error) {
      resolve({ exitCode: null, signal: null, durationMs: Date.now() - started, lines, error: String(error) });
      return;
    }

    child.stdout.on("data", onData);
    child.stderr.on("data", onData);
    child.on("error", (error) => {
      resolve({ exitCode: null, signal: null, durationMs: Date.now() - started, lines, error: error.message });
    });
    child.on("close", (exitCode, signal) => {
      if (partial) lines.push(partial);
      partial = "";
      resolve({ exitCode, signal, durationMs: Date.now() - started, lines, error: null });
    });
  });
}

function tailOf(lines) {
  const trimmed = [...lines];
  while (trimmed.length && trimmed[trimmed.length - 1].trim() === "") trimmed.pop();
  return trimmed.slice(-TAIL_LINES);
}

function formatSeconds(ms) {
  return `${(ms / 1000).toFixed(1)}s`;
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  const steps = selectSteps(opts);
  if (steps.length === 0) {
    console.error("No steps selected.");
    process.exit(2);
  }

  const startedAt = new Date();
  const results = [];
  let failed = false;

  for (const step of steps) {
    if (failed && !opts.keepGoing) {
      results.push({ id: step.id, name: step.name, command: step.cmd, status: "skipped" });
      console.log(`SKIP ${step.name} (not run after an earlier failure)`);
      continue;
    }

    console.log(`RUN  ${step.name}: ${step.cmd}`);
    const result = await runStep(step);
    const passed = !result.error && result.exitCode === 0;
    const tail = passed ? [] : tailOf(result.lines);
    const duration = formatSeconds(result.durationMs);

    if (passed) {
      console.log(`PASS ${step.name} (${duration})`);
    } else {
      const reason = result.error
        ? `spawn error: ${result.error}`
        : result.exitCode !== null
          ? `exit ${result.exitCode}`
          : `signal ${result.signal}`;
      console.log(`FAIL ${step.name} (${reason}) after ${duration}`);
      console.log(`---- last ${tail.length} lines of ${step.name} ----`);
      for (const line of tail) console.log(line);
      console.log(`---- end ${step.name} ----`);
      failed = true;
    }

    results.push({
      id: step.id,
      name: step.name,
      command: step.cmd,
      status: passed ? "pass" : "fail",
      exitCode: result.exitCode,
      signal: result.signal,
      durationMs: result.durationMs,
      error: result.error,
      tail,
    });
  }

  const ok = !failed && results.every((r) => r.status === "pass");
  console.log(ok ? "CI check: all steps passed" : "CI check: FAILED");

  if (opts.json) {
    const dir = join(ROOT, ".ci");
    await mkdir(dir, { recursive: true });
    const summary = {
      ok,
      startedAt: startedAt.toISOString(),
      finishedAt: new Date().toISOString(),
      steps: results,
    };
    await writeFile(join(dir, "last-run.json"), JSON.stringify(summary, null, 2) + "\n", "utf8");
    console.log("Wrote .ci/last-run.json");
  }

  process.exit(ok ? 0 : 1);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
