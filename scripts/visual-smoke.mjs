#!/usr/bin/env node
import { spawn } from "node:child_process";
import { mkdir, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outputDir = path.join(root, "output", "visual-smoke");
const port = Number(process.env.VISUAL_SMOKE_PORT ?? 4173);
const baseUrl = `http://127.0.0.1:${port}`;

const screens = ["dashboard", "power", "eumetsat", "ndvi", "pv", "saved", "api", "settings", "about"];
const viewports = [
  { width: 1024, height: 720 },
  { width: 1280, height: 853 },
  { width: 1440, height: 900 },
];

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
  const server = spawn("npm", ["run", "preview", "--", "--port", String(port), "--strictPort"], {
    cwd: root,
    env: process.env,
    stdio: ["ignore", "pipe", "pipe"],
    shell: process.platform === "win32",
  });
  server.stdout.on("data", (chunk) => process.stdout.write(chunk));
  server.stderr.on("data", (chunk) => process.stderr.write(chunk));
  return server;
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

async function capturePage(browser, screen, viewport) {
  const page = await browser.newPage({
    viewport,
    reducedMotion: "reduce",
  });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  await page.goto(`${baseUrl}/#${screen}`, { waitUntil: "networkidle" });
  await page.locator(".app-shell").waitFor({ state: "visible", timeout: 10_000 });
  await page.waitForTimeout(150);

  const metrics = await page.evaluate(() => ({
    textLength: document.body.innerText.trim().length,
    scrollWidth: document.documentElement.scrollWidth,
    clientWidth: document.documentElement.clientWidth,
    activeNav: document.querySelector(".nav-item.active strong")?.textContent?.trim() ?? "",
    heading: document.querySelector(".screen-heading h1")?.textContent?.trim() ?? "",
  }));

  const failures = [];
  if (errors.length > 0) {
    failures.push(`console/page errors: ${errors.join("; ")}`);
  }
  if (metrics.textLength < 100) {
    failures.push(`page text is unexpectedly short (${metrics.textLength} chars)`);
  }
  if (metrics.scrollWidth > metrics.clientWidth + 2) {
    failures.push(`global horizontal overflow (${metrics.scrollWidth}px > ${metrics.clientWidth}px)`);
  }
  if (!metrics.activeNav || !metrics.heading) {
    failures.push(`missing active navigation or screen heading (${metrics.activeNav || "no nav"} / ${metrics.heading || "no heading"})`);
  }

  const fileName = `${screen}-${viewport.width}x${viewport.height}.png`;
  const screenshot = await page.screenshot({
    path: path.join(outputDir, fileName),
    fullPage: false,
    animations: "disabled",
  });
  if (screenshot.byteLength < 10_000) {
    failures.push(`screenshot is unexpectedly small (${screenshot.byteLength} bytes)`);
  }

  await page.close();
  return failures.map((failure) => `${screen} ${viewport.width}x${viewport.height}: ${failure}`);
}

let server;
let browser;
try {
  await run("npm", ["run", "build"]);
  await rm(outputDir, { recursive: true, force: true });
  await mkdir(outputDir, { recursive: true });

  server = startPreview();
  await waitForPreview(server);

  browser = await chromium.launch({ headless: true });
  const failures = [];
  for (const viewport of viewports) {
    for (const screen of screens) {
      failures.push(...(await capturePage(browser, screen, viewport)));
    }
  }

  if (failures.length > 0) {
    throw new Error(`Visual smoke failed:\n${failures.join("\n")}`);
  }

  console.log(`Visual smoke captured ${screens.length * viewports.length} screenshots in ${path.relative(root, outputDir)}`);
} finally {
  await browser?.close();
  if (server && server.exitCode === null) {
    server.kill();
  }
}
