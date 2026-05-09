import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const bashAvailable = spawnSync("bash", ["--version"], { encoding: "utf8" }).status === 0;
const describeWithBash = bashAvailable ? describe : describe.skip;
const scriptPath = resolve("scripts/check-release-secrets.sh");

function runPreflight(env: Record<string, string> = {}) {
  return spawnSync("bash", [scriptPath], {
    encoding: "utf8",
    env: {
      PATH: process.env.PATH ?? "",
      ...env,
    },
  });
}

const baseSigningSecrets = {
  WINDOWS_SIGN_COMMAND: "signtool sign /fd SHA256 /a {file}",
  APPLE_CERTIFICATE: "base64-p12",
  APPLE_CERTIFICATE_PASSWORD: "certificate-password",
  KEYCHAIN_PASSWORD: "temporary-keychain-password",
};

describeWithBash("release credential preflight", () => {
  it("fails closed when production signing secrets are absent", () => {
    const result = runPreflight();

    expect(result.status).not.toBe(0);
    expect(result.stderr).toContain("Refusing to publish a public release");
    expect(result.stderr).toContain("WINDOWS_SIGN_COMMAND");
    expect(result.stderr).toContain("APPLE_CERTIFICATE");
    expect(result.stderr).toContain("KEYCHAIN_PASSWORD");
  });

  it("requires one complete Apple notarization credential set", () => {
    const result = runPreflight(baseSigningSecrets);

    expect(result.status).not.toBe(0);
    expect(result.stderr).toContain(
      "APPLE_ID/APPLE_PASSWORD/APPLE_TEAM_ID or APPLE_API_KEY/APPLE_API_ISSUER/APPLE_API_KEY_P8_BASE64",
    );
  });

  it("passes with Apple ID notarization credentials", () => {
    const result = runPreflight({
      ...baseSigningSecrets,
      APPLE_ID: "release@example.com",
      APPLE_PASSWORD: "app-specific-password",
      APPLE_TEAM_ID: "TEAMID1234",
    });

    expect(result.status).toBe(0);
    expect(result.stdout).toContain("Release signing/notarization secret preflight passed.");
    expect(result.stderr).toBe("");
  });

  it("passes with App Store Connect API notarization credentials", () => {
    const result = runPreflight({
      ...baseSigningSecrets,
      APPLE_API_KEY: "ABC123DEFG",
      APPLE_API_ISSUER: "issuer-uuid",
      APPLE_API_KEY_P8_BASE64: "base64-p8",
    });

    expect(result.status).toBe(0);
    expect(result.stdout).toContain("Release signing/notarization secret preflight passed.");
    expect(result.stderr).toBe("");
  });
});
