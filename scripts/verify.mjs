import { spawnSync } from "node:child_process";

const isWindows = process.platform === "win32";
const env = { ...process.env };

if (!isWindows) {
  env.PATH = `/opt/homebrew/opt/rustup/bin:${env.PATH ?? ""}`;
}

const executable = (command) => (isWindows && command === "npm" ? "npm.cmd" : command);

const commands = [
  ["npm", ["run", "typecheck"]],
  ["npm", ["run", "test"]],
  ["npm", ["run", "build"]],
  ["cargo", ["fmt", "--all", "--", "--check"]],
  ["cargo", ["test", "--workspace", "--locked"]],
  ["cargo", ["check", "--workspace", "--locked"]],
  ["cargo", ["clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"]],
  ["npm", ["audit", "--omit=dev"]],
];

for (const [command, args] of commands) {
  console.log(`\n> ${command} ${args.join(" ")}`);
  const result = spawnSync(executable(command), args, {
    env,
    shell: false,
    stdio: "inherit",
  });

  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
