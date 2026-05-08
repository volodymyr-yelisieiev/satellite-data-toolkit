import { createHash } from "node:crypto";
import { createWriteStream } from "node:fs";
import { chmod, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { get } from "node:https";
import { arch, platform, tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const releaseVersion = "3.1.1";
const projectRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const tauriDir = join(projectRoot, "src-tauri");
const binaryDir = join(tauriDir, "binaries");
const resourceDir = join(tauriDir, "resources");
const manifestPath = join(resourceDir, "eumdac-sidecar-manifest.json");
const generatedConfigPath = join(tauriDir, "tauri.eumdac.generated.conf.json");

const sidecars = {
  "aarch64-apple-darwin": {
    platform: "darwin",
    arch: "arm64",
    outputName: "eumdac-aarch64-apple-darwin",
    runtimeAlias: "eumdac",
    archiveSha256: "200fb9ece8d790f1314b1ba08a03009b19836764d5312077f5ff18f34774cd3a",
    binarySha256: "09cff6055e4c590fd890d1dc9c93ca32e7037552536ba908b4f7b5c90a2150a2",
    binaryInArchive: "eumdac",
    url: "https://gitlab.eumetsat.int/-/project/3344/uploads/3ef472c576fe04a4d5a80e20a2d7ea88/eumdac-3.1.1-macos-arm64.zip",
  },
  "x86_64-apple-darwin": {
    platform: "darwin",
    arch: "x64",
    outputName: "eumdac-x86_64-apple-darwin",
    runtimeAlias: "eumdac",
    archiveSha256: "ca3f0bbba67003bb2fd91dcce90b7961543bb5b2f312ce882eb6094858d466ca",
    binarySha256: "e969beeb3d7c22b6149696b08add6032078b1472baf5afc46a366e0710f035d2",
    binaryInArchive: "eumdac",
    url: "https://gitlab.eumetsat.int/-/project/3344/uploads/4ca2a5db313e1a07cc47dc1c9afef90b/eumdac-3.1.1-macos-x86_64.zip",
  },
  "x86_64-pc-windows-msvc": {
    platform: "win32",
    arch: "x64",
    outputName: "eumdac-x86_64-pc-windows-msvc.exe",
    runtimeAlias: "eumdac.exe",
    archiveSha256: "844f16dc63accd34e1b013afbbaa6418f40ff06f852901aa25478a46b59eb80b",
    binarySha256: "cf3cde0fd3dc2c57c51996783b8bc53418efa52bc5282fbbadec11f4310613f3",
    binaryInArchive: "eumdac.exe",
    url: "https://gitlab.eumetsat.int/-/project/3344/uploads/c5cdc662a3d4f582ed7bfd52312fb897/eumdac-3.1.1-windows.zip",
  },
};

function selectedSidecars() {
  if (process.argv.includes("--all")) {
    return Object.entries(sidecars);
  }
  const currentPlatform = platform();
  const currentArch = arch();
  const matches = Object.entries(sidecars).filter(
    ([, sidecar]) => sidecar.platform === currentPlatform && sidecar.arch === currentArch,
  );
  if (matches.length === 0) {
    throw new Error(`No EUMDAC sidecar is pinned for ${currentPlatform}/${currentArch}`);
  }
  return matches;
}

async function cleanGeneratedFiles() {
  await mkdir(binaryDir, { recursive: true });
  await mkdir(resourceDir, { recursive: true });
  for (const entry of await readdir(binaryDir).catch(() => [])) {
    if (entry.startsWith("eumdac")) {
      await rm(join(binaryDir, entry), { force: true });
    }
  }
  await rm(manifestPath, { force: true });
  await rm(generatedConfigPath, { force: true });
}

async function download(url, destination, redirects = 0) {
  if (redirects > 5) {
    throw new Error(`Too many redirects while downloading ${url}`);
  }
  await new Promise((resolvePromise, reject) => {
    const request = get(url, (response) => {
      if (response.statusCode && response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        response.resume();
        download(new URL(response.headers.location, url).toString(), destination, redirects + 1).then(resolvePromise, reject);
        return;
      }
      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`Download failed with HTTP ${response.statusCode}: ${url}`));
        return;
      }
      const file = createWriteStream(destination, { mode: 0o644 });
      response.pipe(file);
      file.on("finish", () => file.close(resolvePromise));
      file.on("error", reject);
    });
    request.on("error", reject);
  });
}

async function sha256File(path) {
  const hash = createHash("sha256");
  hash.update(await readFile(path));
  return hash.digest("hex");
}

function run(command, args) {
  const result = spawnSync(command, args, { stdio: "inherit" });
  return result.status === 0;
}

function extractZip(archivePath, extractDir) {
  if (run("tar", ["-xf", archivePath, "-C", extractDir])) {
    return;
  }
  if (platform() === "win32") {
    const command = `Expand-Archive -LiteralPath '${archivePath.replaceAll("'", "''")}' -DestinationPath '${extractDir.replaceAll("'", "''")}' -Force`;
    if (run("powershell", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", command])) {
      return;
    }
  }
  if (run("unzip", ["-q", archivePath, "-d", extractDir])) {
    return;
  }
  throw new Error(`Could not extract ${archivePath}`);
}

async function stageSidecar([triple, sidecar]) {
  const workDir = join(tmpdir(), `satellite-eumdac-${process.pid}-${triple}`);
  const archivePath = join(workDir, basename(sidecar.url));
  const extractDir = join(workDir, "extract");
  await rm(workDir, { force: true, recursive: true });
  await mkdir(extractDir, { recursive: true });

  console.log(`Downloading EUMDAC ${releaseVersion} for ${triple}`);
  await download(sidecar.url, archivePath);
  const archiveSha256 = await sha256File(archivePath);
  if (archiveSha256 !== sidecar.archiveSha256) {
    throw new Error(`EUMDAC archive checksum mismatch for ${triple}: ${archiveSha256}`);
  }

  extractZip(archivePath, extractDir);
  const extractedBinary = join(extractDir, sidecar.binaryInArchive);
  const binarySha256 = await sha256File(extractedBinary);
  if (binarySha256 !== sidecar.binarySha256) {
    throw new Error(`EUMDAC binary checksum mismatch for ${triple}: ${binarySha256}`);
  }

  const outputPath = join(binaryDir, sidecar.outputName);
  await rm(outputPath, { force: true });
  await writeFile(outputPath, await readFile(extractedBinary), { mode: 0o755 });
  if (sidecar.platform !== "win32") {
    await chmod(outputPath, 0o755);
  }
  await rm(workDir, { force: true, recursive: true });
  console.log(`Staged ${outputPath}`);
  return { triple, ...sidecar };
}

function manifestEntries(stagedSidecars) {
  return stagedSidecars.flatMap((sidecar) => {
    const common = {
      sha256: sidecar.binarySha256,
      version: releaseVersion,
      source: sidecar.url,
      license: "MIT",
    };
    return [
      { name: sidecar.outputName, ...common },
      { name: sidecar.runtimeAlias, ...common },
    ];
  });
}

async function writeGeneratedConfig() {
  const config = {
    bundle: {
      externalBin: ["binaries/eumdac"],
      resources: {
        "resources/eumdac-sidecar-manifest.json": "eumdac-sidecar-manifest.json",
      },
    },
  };
  await writeFile(generatedConfigPath, `${JSON.stringify(config, null, 2)}\n`);
}

await cleanGeneratedFiles();
const staged = [];
for (const sidecar of selectedSidecars()) {
  staged.push(await stageSidecar(sidecar));
}
await writeFile(
  manifestPath,
  `${JSON.stringify({ binaries: manifestEntries(staged) }, null, 2)}\n`,
);
await writeGeneratedConfig();

console.log(`Generated ${manifestPath}`);
console.log(`Generated ${generatedConfigPath}`);
