import { spawn, spawnSync } from "node:child_process";
import { mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { accounts } from "./accounts.mjs";

const database = process.env.STUCCO_E2E_DATABASE;
if (!database) throw new Error("The full-stack Playwright config must supply an isolated database path.");
const directory = dirname(database);
// Refuse to reuse an existing directory; this harness only removes what it creates.
mkdirSync(directory);
const bundle = process.env.STUCCO_E2E_BUNDLE;
const binary = bundle ? resolve(bundle, "orders-fullstack") : resolve("target/debug/orders-fullstack");
let cleaned = false;
function cleanup() {
  if (!cleaned) {
    cleaned = true;
    rmSync(directory, { recursive: true, force: true });
  }
}
process.on("exit", cleanup);

function cli(args, password) {
  const result = spawnSync(binary, ["--database", database, ...args], {
    input: password === undefined ? undefined : `${password}\n`,
    encoding: "utf8",
    env: process.env,
    cwd: bundle,
  });
  if (result.error || result.status !== 0) {
    throw new Error(`Browser database setup failed: ${result.error?.message ?? result.stderr}`);
  }
}

for (const account of Object.values(accounts)) {
  cli(["user-add", "--tenant", account.tenant, "--username", account.username, "--display-name", account.displayName], account.password);
}
// Preserve the existing suite's owned seed IDs: Alice's orders are created first.
cli(["seed", "--username", accounts.alice.username]);
cli(["seed", "--username", accounts.bob.username]);

const child = spawn(binary, ["--database", database, "serve"], { stdio: "inherit", env: process.env, cwd: bundle });
for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => child.kill(signal));
}
child.on("error", (error) => {
  console.error(error.message);
  process.exitCode = 1;
});
child.on("exit", (code) => {
  cleanup();
  process.exitCode = code ?? 0;
});
