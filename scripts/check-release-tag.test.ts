import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { describe, expect, it } from "vitest";

const bashAvailable = spawnSync("bash", ["--version"], { encoding: "utf8" }).status === 0;
const gitAvailable = spawnSync("git", ["--version"], { encoding: "utf8" }).status === 0;
const describeWithRuntime = bashAvailable && gitAvailable ? describe : describe.skip;
const scriptPath = resolve("scripts/check-release-tag.sh");

function createRepo(version = "2.1.1") {
  const cwd = mkdtempSync(join(tmpdir(), "satellite-release-tag-"));
  spawnSync("git", ["init"], { cwd, stdio: "ignore" });
  spawnSync("git", ["config", "user.email", "codex@example.com"], { cwd, stdio: "ignore" });
  spawnSync("git", ["config", "user.name", "Codex"], { cwd, stdio: "ignore" });
  writeFileSync(join(cwd, "package.json"), `${JSON.stringify({ version }, null, 2)}\n`, "utf8");
  spawnSync("git", ["add", "package.json"], { cwd, stdio: "ignore" });
  spawnSync("git", ["commit", "-m", "seed"], { cwd, stdio: "ignore" });
  return cwd;
}

function runPreflight(cwd: string, tag: string) {
  return spawnSync("bash", [scriptPath, tag], {
    cwd,
    encoding: "utf8",
    env: {
      PATH: process.env.PATH ?? "",
    },
  });
}

describeWithRuntime("release tag preflight", () => {
  it("passes for an existing v-prefixed tag that matches package.json", () => {
    const cwd = createRepo();
    try {
      spawnSync("git", ["tag", "v2.1.1"], { cwd, stdio: "ignore" });

      const result = runPreflight(cwd, "v2.1.1");

      expect(result.status).toBe(0);
      expect(result.stdout).toContain("Release tag preflight passed: v2.1.1");
      expect(result.stderr).toBe("");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  it("rejects tags that do not exist locally", () => {
    const cwd = createRepo();
    try {
      const result = runPreflight(cwd, "v2.1.1");

      expect(result.status).not.toBe(0);
      expect(result.stderr).toContain("Release tag does not exist locally: v2.1.1");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  it("rejects non-release-shaped tags", () => {
    const cwd = createRepo();
    try {
      spawnSync("git", ["tag", "release-2.1.1"], { cwd, stdio: "ignore" });

      const result = runPreflight(cwd, "release-2.1.1");

      expect(result.status).not.toBe(0);
      expect(result.stderr).toContain("Release tag must look like vX.Y.Z");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  it("rejects an existing tag that does not match package.json version", () => {
    const cwd = createRepo("2.1.1");
    try {
      spawnSync("git", ["tag", "v2.1.2"], { cwd, stdio: "ignore" });

      const result = runPreflight(cwd, "v2.1.2");

      expect(result.status).not.toBe(0);
      expect(result.stderr).toContain("expected v2.1.1");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });
});
