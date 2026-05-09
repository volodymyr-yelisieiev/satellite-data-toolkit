import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { afterEach, describe, expect, test } from "vitest";

const checker = resolve("scripts/check-tauri-surface.mjs");
const fixtureRoots: string[] = [];

function writeFixture(files: Record<string, string>) {
  const root = mkdtempSync(join(tmpdir(), "tauri-surface-"));
  fixtureRoots.push(root);
  for (const [path, content] of Object.entries({
    "package.json": JSON.stringify({ dependencies: {}, devDependencies: {} }),
    "src/app.ts": "import { invoke } from '@tauri-apps/api/core';\nvoid invoke;\n",
    "src-tauri/Cargo.toml": "[package]\nname = \"surface-fixture\"\nversion = \"0.0.0\"\n",
    "src-tauri/capabilities/default.json": JSON.stringify({
      identifier: "main-window",
      windows: ["main"],
      permissions: ["core:default"],
    }),
    ...files,
  })) {
    const target = join(root, path);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, content);
  }
  return root;
}

function runChecker(root: string) {
  return spawnSync(process.execPath, [checker], {
    cwd: root,
    encoding: "utf8",
  });
}

describe("check-tauri-surface", () => {
  afterEach(() => {
    while (fixtureRoots.length > 0) {
      rmSync(fixtureRoots.pop()!, { recursive: true, force: true });
    }
  });

  test("accepts the reviewed core-only surface", () => {
    const result = runChecker(writeFixture({}));

    expect(result.status).toBe(0);
    expect(result.stdout).toContain("Tauri API surface is constrained");
  });

  test("rejects frontend Tauri plugin imports", () => {
    const result = runChecker(
      writeFixture({
        "src/app.ts": "import { open } from '@tauri-apps/plugin-dialog';\nvoid open;\n",
      }),
    );

    expect(result.status).toBe(1);
    expect(result.stderr).toContain("plugin-dialog");
    expect(result.stderr).toContain("Tauri plugin APIs must be reviewed");
  });

  test("rejects remote capability grants", () => {
    const result = runChecker(
      writeFixture({
        "src-tauri/capabilities/default.json": JSON.stringify({
          identifier: "main-window",
          windows: ["main"],
          permissions: ["core:default"],
          remote: { urls: ["https://example.com"] },
        }),
      }),
    );

    expect(result.status).toBe(1);
    expect(result.stderr).toContain("must not grant capability access to remote URLs");
  });
});
