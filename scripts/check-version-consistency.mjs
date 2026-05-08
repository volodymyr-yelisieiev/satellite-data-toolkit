import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const isWindows = process.platform === "win32";
const env = { ...process.env };

if (!isWindows) {
  env.PATH = `/opt/homebrew/opt/rustup/bin:${env.PATH ?? ""}`;
}

function readJson(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    throw new Error(`Failed to read ${path}: ${error.message}`);
  }
}

function readCargoMetadata() {
  const result = spawnSync("cargo", ["metadata", "--format-version=1", "--locked", "--no-deps"], {
    encoding: "utf8",
    env,
    shell: isWindows,
  });

  if (result.error) {
    throw new Error(`Failed to run cargo metadata: ${result.error.message}`);
  }

  if (result.status !== 0) {
    throw new Error(result.stderr || `cargo metadata failed with exit code ${result.status}`);
  }

  try {
    return JSON.parse(result.stdout);
  } catch (error) {
    throw new Error(`Failed to parse cargo metadata output: ${error.message}`);
  }
}

const packageJson = readJson("package.json");
const packageLock = readJson("package-lock.json");
const tauriConfig = readJson("src-tauri/tauri.conf.json");
const cargoMetadata = readCargoMetadata();

const versions = new Map([
  ["package.json", packageJson.version],
  ["package-lock.json", packageLock.version],
  ["package-lock.json packages['']", packageLock.packages?.[""]?.version],
  ["src-tauri/tauri.conf.json", tauriConfig.version],
]);

const workspaceMembers = new Set(cargoMetadata.workspace_members);
for (const cargoPackage of cargoMetadata.packages) {
  if (workspaceMembers.has(cargoPackage.id)) {
    versions.set(`Cargo package ${cargoPackage.name}`, cargoPackage.version);
  }
}

const missing = [...versions].filter(([, version]) => typeof version !== "string" || version.length === 0);
if (missing.length > 0) {
  console.error("Missing version metadata:");
  for (const [name] of missing) {
    console.error(`- ${name}`);
  }
  process.exit(1);
}

const uniqueVersions = new Set(versions.values());
if (uniqueVersions.size !== 1) {
  console.error("Version metadata is inconsistent:");
  for (const [name, version] of versions) {
    console.error(`- ${name}: ${version}`);
  }
  process.exit(1);
}

console.log(`Version metadata is consistent: ${versions.values().next().value}`);
