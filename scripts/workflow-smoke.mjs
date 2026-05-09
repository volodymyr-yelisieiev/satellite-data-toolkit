#!/usr/bin/env node
import { spawn } from "node:child_process";
import { mkdir, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outputDir = path.join(root, "output", "workflow-smoke");
const port = Number(process.env.WORKFLOW_SMOKE_PORT ?? 4174);
const baseUrl = `http://127.0.0.1:${port}`;
const viteBin = path.join(root, "node_modules", ".bin", process.platform === "win32" ? "vite.cmd" : "vite");

function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: root,
      env: process.env,
      stdio: "inherit",
      shell: process.platform === "win32",
      ...options,
    });
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`${command} ${args.join(" ")} exited with ${code}`));
    });
  });
}

function startPreview() {
  const server = spawn(viteBin, ["preview", "--host", "127.0.0.1", "--port", String(port), "--strictPort"], {
    cwd: root,
    env: process.env,
    stdio: ["ignore", "pipe", "pipe"],
    detached: process.platform !== "win32",
    shell: process.platform === "win32",
  });
  server.stdout.on("data", (chunk) => process.stdout.write(chunk));
  server.stderr.on("data", (chunk) => process.stderr.write(chunk));
  return server;
}

function stopPreview(server) {
  if (server.exitCode !== null) return;
  if (process.platform === "win32") {
    server.kill();
  } else {
    process.kill(-server.pid, "SIGTERM");
  }
}

async function waitForPreview(server) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 30_000) {
    if (server.exitCode !== null) {
      throw new Error(`vite preview exited early with ${server.exitCode}`);
    }
    try {
      const response = await fetch(baseUrl);
      if (response.ok) return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
  }
  throw new Error(`vite preview did not respond at ${baseUrl}`);
}

async function expectVisible(locator, label) {
  try {
    await locator.first().waitFor({ state: "visible", timeout: 10_000 });
  } catch (error) {
    throw new Error(`Expected visible: ${label}. ${error.message}`);
  }
}

async function expectHidden(locator, label) {
  try {
    await locator.first().waitFor({ state: "hidden", timeout: 10_000 });
  } catch (error) {
    throw new Error(`Expected hidden: ${label}. ${error.message}`);
  }
}

async function clickButton(page, name) {
  await page.getByRole("button", { name }).click();
}

async function runWorkflow(page) {
  await page.goto(`${baseUrl}/#power`, { waitUntil: "domcontentloaded", timeout: 15_000 });
  await page.locator(".app-shell").waitFor({ state: "visible", timeout: 10_000 });

  await clickButton(page, /Fetch Data/i);
  await expectVisible(page.getByText("Success"), "NASA POWER success badge");
  await expectVisible(page.getByText("2024-05-01"), "NASA POWER first record");
  await clickButton(page, /^Save$/i);
  await expectVisible(page.getByText(/Dataset saved locally/i), "saved dataset log entry");
  await clickButton(page, /Export CSV/i);
  await expectVisible(page.getByText(/Last export: .*nasa_power_demo\.csv/i), "direct CSV export path");

  await page.getByRole("tab", { name: /PV Estimate/i }).click();
  await clickButton(page, /Estimate Local PV/i);
  await expectVisible(page.getByText("Energy"), "local PV energy metric");
  await clickButton(page, /PVWatts\/NLR/i);
  await expectVisible(page.getByText("PVWatts AC Annual"), "PVWatts annual metric");

  await page.getByRole("button", { name: /Saved Data/i }).click();
  await expectVisible(page.getByText(/NASA POWER 2024-05-01 to 2024-05-31/i), "saved dataset row");
  await clickButton(page, /^JSON$/i);
  await expectVisible(page.getByText(/Last export: .*nasa_power_demo\.csv/i), "saved dataset export path");

  await page.getByRole("tab", { name: /EUMETSAT/i }).click();
  await clickButton(page, /Check Sidecar/i);
  await expectVisible(page.getByText("Missing"), "missing sidecar badge");
  await clickButton(page, /Search Products/i);
  await expectVisible(page.getByText("Demo EUMETSAT Product"), "EUMETSAT demo product");
  await page.getByLabel("Output Directory").fill("/demo/downloads");
  await clickButton(page, /Download Selected/i);
  await expectVisible(page.getByText(/EUMETSAT product download completed/i), "EUMETSAT download log");

  await page.getByRole("tab", { name: /NDVI Calculator/i }).click();
  await page.getByLabel("Red Band TIFF").fill("/demo/red.tif");
  await page.getByLabel("NIR Band TIFF").fill("/demo/nir.tif");
  await clickButton(page, /^Validate$/i);
  await expectVisible(page.getByText(/NDVI job is structurally valid/i), "NDVI validation summary");
  await clickButton(page, /Run NDVI/i);
  await expectVisible(page.getByText("NDVI Result"), "NDVI result panel");

  await page.getByRole("button", { name: /API Slots/i }).click();
  const pvWattsCard = page.locator(".api-card").filter({ hasText: "PVWatts/NLR Key" });
  await pvWattsCard.getByPlaceholder("Paste key, then Store").fill("demo-key");
  await pvWattsCard.getByRole("button", { name: /^Store$/i }).click();
  await expectVisible(pvWattsCard.getByText("Stored"), "stored PVWatts credential state");
  await pvWattsCard.getByRole("button", { name: /^Test$/i }).click();
  await expectVisible(pvWattsCard.getByText("Demo credential is stored."), "credential test result");
  await pvWattsCard.getByRole("button", { name: /^Delete$/i }).click();
  await expectVisible(pvWattsCard.getByText("Empty"), "deleted credential state");
  await expectHidden(pvWattsCard.getByText("Demo credential is stored."), "stale credential test result");
}

let server;
let browser;
try {
  await run("npm", ["run", "build"]);
  await rm(outputDir, { recursive: true, force: true });
  await mkdir(outputDir, { recursive: true });

  server = startPreview();
  await waitForPreview(server);

  browser = await chromium.launch({ args: ["--disable-dev-shm-usage"], headless: true });
  const page = await browser.newPage({
    viewport: { width: 1280, height: 853 },
    reducedMotion: "reduce",
  });
  page.setDefaultTimeout(15_000);

  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  await runWorkflow(page);

  if (errors.length > 0) {
    throw new Error(`Browser workflow smoke saw console/page errors:\n${errors.join("\n")}`);
  }

  await page.screenshot({
    path: path.join(outputDir, "workflow-1280x853.png"),
    fullPage: false,
    animations: "disabled",
  });
  await page.close();
  console.log(`Browser workflow smoke passed; screenshot saved in ${path.relative(root, outputDir)}`);
} finally {
  await browser?.close();
  if (server && server.exitCode === null) {
    stopPreview(server);
  }
}
