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
const viteBin = path.join(root, "node_modules", ".bin", process.platform === "win32" ? "vite.cmd" : "vite");

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

async function capturePage(browser, screen, viewport) {
  const page = await browser.newPage({
    viewport,
    reducedMotion: "reduce",
  });
  page.setDefaultTimeout(15_000);
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  await page.goto(`${baseUrl}/#${screen}`, { waitUntil: "domcontentloaded", timeout: 15_000 });
  await page.waitForLoadState("networkidle", { timeout: 5_000 }).catch(() => undefined);
  await page.locator(".app-shell").waitFor({ state: "visible", timeout: 10_000 });
  await page.waitForTimeout(150);

  const metrics = await page.evaluate(() => {
    const overflowSelectors = [
      ".nav-item",
      ".top-tab",
      ".doc-button",
      ".primary-action",
      ".secondary-action",
      ".icon-button",
      ".status-badge",
      ".dashboard-tile",
      ".api-card",
      "input",
      "select",
    ];
    const overflowIssues = Array.from(document.querySelectorAll(overflowSelectors.join(","))).flatMap((element) => {
      if (!(element instanceof HTMLElement)) return [];
      const style = window.getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      if (style.display === "none" || style.visibility === "hidden" || rect.width < 1 || rect.height < 1) return [];
      const horizontalOverflow = element.scrollWidth > Math.ceil(element.clientWidth) + 2;
      const verticalOverflow = element.scrollHeight > Math.ceil(element.clientHeight) + 2;
      const text = element.innerText.trim() || element.getAttribute("placeholder") || element.getAttribute("aria-label") || element.tagName.toLowerCase();
      const label = text.replace(/\s+/g, " ").slice(0, 80);
      const issues = [];
      if (horizontalOverflow || verticalOverflow) {
        issues.push(`clipped control "${label}" (${element.scrollWidth}x${element.scrollHeight} > ${element.clientWidth}x${element.clientHeight})`);
      }
      const container = element.closest(".card,.content-panel,.footer,.top-tabs");
      if (container instanceof HTMLElement) {
        const containerRect = container.getBoundingClientRect();
        if (rect.left < containerRect.left - 2 || rect.right > containerRect.right + 2) {
          issues.push(`control outside container "${label}" (${Math.round(rect.left)}..${Math.round(rect.right)} outside ${Math.round(containerRect.left)}..${Math.round(containerRect.right)})`);
        }
      }
      return issues;
    });
    return {
      textLength: document.body.innerText.trim().length,
      scrollWidth: document.documentElement.scrollWidth,
      clientWidth: document.documentElement.clientWidth,
      activeNav: document.querySelector(".nav-item.active strong")?.textContent?.trim() ?? "",
      heading: document.querySelector(".screen-heading h1")?.textContent?.trim() ?? "",
      overflowIssues: overflowIssues.slice(0, 8),
    };
  });

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
  failures.push(...metrics.overflowIssues);

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

  browser = await chromium.launch({ args: ["--disable-dev-shm-usage"], headless: true });
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
    stopPreview(server);
  }
}
