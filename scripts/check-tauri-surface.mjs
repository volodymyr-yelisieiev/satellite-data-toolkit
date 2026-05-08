import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const root = process.cwd();
const allowedTauriApiImports = new Set(["@tauri-apps/api/core"]);
const sourceExtensions = new Set([".js", ".jsx", ".mjs", ".ts", ".tsx"]);
const failures = [];

function fail(message) {
  failures.push(message);
}

function walk(dir) {
  return readdirSync(dir)
    .flatMap((entry) => {
      const path = join(dir, entry);
      const stat = statSync(path);
      if (stat.isDirectory()) {
        return walk(path);
      }
      return [path];
    });
}

function isSourceFile(path) {
  return [...sourceExtensions].some((extension) => path.endsWith(extension));
}

function checkFrontendImports() {
  const sourceFiles = walk(join(root, "src")).filter(isSourceFile);
  const importPattern = /(?:import\s+(?:[^'"]+?\s+from\s+)?|import\s*\()\s*['"]([^'"]+)['"]/g;

  for (const file of sourceFiles) {
    const text = readFileSync(file, "utf8");
    for (const match of text.matchAll(importPattern)) {
      const specifier = match[1];
      if (specifier.startsWith("@tauri-apps/api/") && !allowedTauriApiImports.has(specifier)) {
        fail(`${relative(root, file)} imports ${specifier}; only @tauri-apps/api/core is allowed.`);
      }
      if (specifier.startsWith("@tauri-apps/plugin-")) {
        fail(`${relative(root, file)} imports ${specifier}; Tauri plugin APIs must be reviewed before use.`);
      }
    }
  }
}

function checkPackageDependencies() {
  const packageJson = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  const dependencyGroups = ["dependencies", "devDependencies", "optionalDependencies"];

  for (const group of dependencyGroups) {
    for (const name of Object.keys(packageJson[group] ?? {})) {
      if (name.startsWith("@tauri-apps/plugin-")) {
        fail(`package.json ${group} includes ${name}; plugin dependencies must be reviewed before use.`);
      }
    }
  }
}

function checkRustDependencies() {
  const cargoToml = readFileSync(join(root, "src-tauri", "Cargo.toml"), "utf8");
  const pluginDependencyPattern = /^\s*tauri-plugin-[a-z0-9_-]+\s*=/m;
  if (pluginDependencyPattern.test(cargoToml)) {
    fail("src-tauri/Cargo.toml includes a tauri-plugin-* dependency; plugin permissions must be reviewed before use.");
  }
}

function checkCapability() {
  const capabilityPath = join(root, "src-tauri", "capabilities", "default.json");
  const capability = JSON.parse(readFileSync(capabilityPath, "utf8"));
  const permissions = capability.permissions ?? [];

  if (capability.identifier !== "main-window") {
    fail("src-tauri/capabilities/default.json must keep identifier main-window.");
  }
  if (JSON.stringify(capability.windows ?? []) !== JSON.stringify(["main"])) {
    fail("src-tauri/capabilities/default.json must only target the main window.");
  }
  if ("remote" in capability || "urls" in capability) {
    fail("src-tauri/capabilities/default.json must not grant capability access to remote URLs.");
  }
  if (JSON.stringify(permissions) !== JSON.stringify(["core:default"])) {
    fail("src-tauri/capabilities/default.json permissions changed; review the Tauri surface before updating this guard.");
  }
}

checkFrontendImports();
checkPackageDependencies();
checkRustDependencies();
checkCapability();

if (failures.length > 0) {
  console.error("Tauri API surface check failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log("Tauri API surface is constrained to reviewed core IPC usage.");
