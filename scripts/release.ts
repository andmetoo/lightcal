#!/usr/bin/env bun
/**
 * Lightcal release build, SimpleSign Desktop signing workflow, and SHA-256 checksum runner.
 *
 * Commands (via Bun or `just`):
 *   bun run scripts/release.ts build    # Build optimized release binary, stage to target/dist/, write SHA-256
 *   bun run scripts/release.ts sign     # Sign with signtool while SimplySign Desktop is signed in
 *   bun run scripts/release.ts verify   # Verify Authenticode signature and update SHA-256
 *   bun run scripts/release.ts release  # Full pipeline: build -> stage -> signtool -> checksum
 *   bun run scripts/release.ts sign-draft vX.Y.Z      # Download that draft, sign on Windows, upload signed files
 *   bun run scripts/release.ts publish-release vX.Y.Z # Publish that draft after the signed assets are attached
 *
 * Signing uses scripts/sign-windows.ps1. Set SIGN_CERT_SHA1 (thumbprint, not a secret) in
 * .env. SimplySign Desktop must be signed in so that certificate is in the user store.
 * SIGNTOOL overrides the signtool.exe path.
 */

import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, join, resolve } from "node:path";
import {
  UNSIGNED_EXE,
  assertChecksum,
  assertFilesExist,
  assertUnsignedDraft,
  assertWindowsExecutable,
  deleteAssetArgs,
  editReleaseArgs,
  missingSignedAssets,
  parseReleaseView,
  publishedReleaseNotes,
  publishedReleaseTitle,
  signedUploadPaths,
  tagForVersion,
  unsignedAssetsPresent,
  uploadArgs,
  versionFromCheckout,
} from "./release-draft.ts";

const ROOT = resolve(import.meta.dir, "..");

for (const name of [".env"]) {
  const path = join(ROOT, name);
  if (existsSync(path)) process.loadEnvFile(path);
}

function stableSemver(raw: string | undefined): string | null {
  if (!raw) return null;
  const match = raw.trim().match(/^v?([0-9]+\.[0-9]+\.[0-9]+)$/);
  return match ? match[1] : null;
}

function releaseVersion(): string {
  const explicit = stableSemver(process.env.LIGHTCAL_RELEASE_VERSION);
  if (explicit) return explicit;
  if (process.env.GITHUB_REF_TYPE === "tag") {
    const tagged = stableSemver(process.env.GITHUB_REF_NAME);
    if (tagged) return tagged;
  }
  const described = Bun.spawnSync(["git", "describe", "--tags", "--exact-match"], {
    cwd: ROOT,
    stdout: "pipe",
    stderr: "ignore",
  });
  if (described.exitCode === 0) {
    const tagged = stableSemver(new TextDecoder().decode(described.stdout));
    if (tagged) return tagged;
  }
  const manifest = readFileSync(join(ROOT, "Cargo.toml"), "utf8");
  const version = manifest.match(/^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"/m)?.[1];
  if (!version) throw new Error("Workspace version is missing or invalid");
  return version;
}
const BIN_NAME = process.platform === "win32" ? "lightcal.exe" : "lightcal";
const RELEASE_BIN = join(ROOT, "target", "release", BIN_NAME);
const DIST_DIR = join(ROOT, "target", "dist");
const DIST_BIN = join(DIST_DIR, BIN_NAME);

function runCmd(cmd: string[], cwd = ROOT, allowFailure = false): number {
  console.log(`> ${cmd.join(" ")}`);
  const proc = Bun.spawnSync(cmd, {
    cwd,
    env: process.env,
    stdout: "inherit",
    stderr: "inherit",
  });
  if (proc.exitCode !== 0 && !allowFailure) {
    process.exit(proc.exitCode ?? 1);
  }
  return proc.exitCode ?? 0;
}

function findSignTool(): string | null {
  const configured = process.env.SIGNTOOL || process.env.SIGNTOOL_PATH;
  if (configured && existsSync(configured)) {
    return configured;
  }
  const kitsRoot = "C:\\Program Files (x86)\\Windows Kits\\10\\bin";
  if (existsSync(kitsRoot)) {
    const versions = readdirSync(kitsRoot)
      .filter((d) => d.startsWith("10."))
      .sort()
      .reverse();
    for (const ver of versions) {
      const candidate = join(kitsRoot, ver, "x64", "signtool.exe");
      if (existsSync(candidate)) {
        return candidate;
      }
    }
  }
  return Bun.which("signtool") ?? null;
}

function buildRelease(): void {
  console.log("=== Building Lightcal release binary ===");
  runCmd(["cargo", "build", "--release", "--workspace", "--locked"]);
  if (!existsSync(RELEASE_BIN)) {
    throw new Error(`Expected release binary not found at ${RELEASE_BIN}`);
  }
  mkdirSync(DIST_DIR, { recursive: true });
  copyFileSync(RELEASE_BIN, DIST_BIN);
}

function writeChecksums(targetPath = DIST_BIN): void {
  mkdirSync(DIST_DIR, { recursive: true });
  if (!existsSync(targetPath) && existsSync(RELEASE_BIN)) {
    copyFileSync(RELEASE_BIN, DIST_BIN);
  }
  const bytes = readFileSync(targetPath);
  const sha256 = createHash("sha256").update(bytes).digest("hex");
  const sizeMiB = (statSync(targetPath).size / (1024 * 1024)).toFixed(2);

  const checksumLine = `${sha256}  ${basename(targetPath)}\n`;
  writeFileSync(`${targetPath}.sha256`, checksumLine, "utf8");

  console.log(`Staged binary: ${targetPath}`);
  console.log(`Payload size:  ${sizeMiB} MiB (${bytes.byteLength} bytes)`);
  console.log(`SHA-256:       ${sha256}`);
}

function signWithSigntool(target = DIST_BIN): void {
  if (process.platform !== "win32") {
    throw new Error("signtool can sign only on the maintainer's Windows PC. Run `just sign-draft` there.");
  }
  if (!existsSync(target) && target !== DIST_BIN) throw new Error(`Missing signing target: ${target}`);
  if (!existsSync(target)) {
    if (existsSync(RELEASE_BIN)) {
      mkdirSync(DIST_DIR, { recursive: true });
      copyFileSync(RELEASE_BIN, DIST_BIN);
    } else {
      buildRelease();
    }
  }

  console.log(`=== Signing ${target} with signtool ===`);
  runCmd(["pwsh", "-NoLogo", "-NoProfile", "-File", join(ROOT, "scripts", "sign-windows.ps1"), "-File", target]);
}

function verifySignature(target = existsSync(DIST_BIN) ? DIST_BIN : RELEASE_BIN): void {
  if (!existsSync(target)) {
    console.error(`Error: No binary found at ${target}. Run 'just build-release' first.`);
    process.exit(1);
  }
  const signtool = findSignTool();
  if (signtool) {
    console.log(`=== Verifying Authenticode signature on ${target} ===`);
    runCmd([signtool, "verify", "/pa", "/v", target]);
  } else {
    console.log("=== Checking Authenticode signature via PowerShell Get-AuthenticodeSignature ===");
    runCmd([
      "pwsh",
      "-NoLogo",
      "-NoProfile",
      "-Command",
      `$sig = Get-AuthenticodeSignature -LiteralPath '${target.replaceAll("'", "''")}'; Format-List -InputObject $sig; if ($sig.Status -ne 'Valid') { exit 1 }`,
    ]);
  }
  writeChecksums(target);
}

function showCertificateFingerprint(): void {
  verifySignature();
  const escaped = DIST_BIN.replaceAll("'", "''");
  runCmd(["pwsh", "-NoLogo", "-NoProfile", "-Command",
    `$sig=Get-AuthenticodeSignature -LiteralPath '${escaped}'; $sha=[Security.Cryptography.SHA256]::Create(); [BitConverter]::ToString($sha.ComputeHash($sig.SignerCertificate.RawData)).Replace('-','')`,
  ]);
}

function packagePortableArchive(): void {
  if (!existsSync(DIST_BIN)) {
    buildRelease();
  }
  writeChecksums(DIST_BIN);

  const version = releaseVersion();
  const pkgDirName = `lightcal-v${version}-windows-x64`;
  const pkgStageDir = join(DIST_DIR, pkgDirName);
  mkdirSync(pkgStageDir, { recursive: true });

  copyFileSync(DIST_BIN, join(pkgStageDir, BIN_NAME));
  copyFileSync(`${DIST_BIN}.sha256`, join(pkgStageDir, `${BIN_NAME}.sha256`));
  for (const docFile of ["LICENSE", "THIRD_PARTY_NOTICES.md", "README.md"]) {
    const src = join(ROOT, docFile);
    if (existsSync(src)) {
      copyFileSync(src, join(pkgStageDir, docFile));
    }
  }

  const zipPath = join(DIST_DIR, `${pkgDirName}.zip`);
  if (process.platform === "win32") {
    runCmd([
      "pwsh",
      "-NoLogo",
      "-NoProfile",
      "-Command",
      `Compress-Archive -Path '${pkgStageDir.replaceAll("'", "''")}\\*' -DestinationPath '${zipPath.replaceAll("'", "''")}' -Force`,
    ]);
  }
  if (existsSync(zipPath)) {
    const zipBytes = readFileSync(zipPath);
    const zipSha = createHash("sha256").update(zipBytes).digest("hex");
    writeFileSync(`${zipPath}.sha256`, `${zipSha}  ${pkgDirName}.zip\n`, "utf8");
    console.log(`Portable archive: ${zipPath}`);
    console.log(`Archive SHA-256:  ${zipSha}`);
  }
}

function signInstaller(): void {
  verifySignature(DIST_BIN);
  const version = releaseVersion();
  runCmd([
    "pwsh",
    "-NoLogo",
    "-NoProfile",
    "-File",
    "scripts/package-nsis.ps1",
    "-SkipBuild",
    "-RequireSigned",
    "-BinaryPath",
    DIST_BIN,
    "-Version",
    version,
  ]);
  const setup = join(ROOT, "target", "nsis", `lightcal-${version}-windows-x64-setup.exe`);
  signWithSigntool(setup);
  verifySignature(setup);
}

function textOf(bytes: Uint8Array): string {
  return new TextDecoder().decode(bytes).trim();
}

function ensureGh(): void {
  const version = Bun.spawnSync(["gh", "--version"], { cwd: ROOT, stdout: "pipe", stderr: "pipe" });
  if (version.exitCode !== 0) {
    throw new Error("gh is not installed. Install GitHub CLI (https://cli.github.com) and run gh auth login.");
  }
  const auth = Bun.spawnSync(["gh", "auth", "status"], { cwd: ROOT, stdout: "pipe", stderr: "pipe" });
  if (auth.exitCode !== 0) {
    throw new Error("gh is not signed in. Run gh auth login, then repeat this command.");
  }
}

function ghJson(args: string[]): string {
  const proc = Bun.spawnSync(["gh", ...args], { cwd: ROOT, stdout: "pipe", stderr: "pipe" });
  if (proc.exitCode !== 0) {
    throw new Error(textOf(proc.stderr) || `gh ${args.join(" ")} failed`);
  }
  return textOf(proc.stdout);
}

function ghInherit(args: string[]): number {
  console.log(`> gh ${args.join(" ")}`);
  const proc = Bun.spawnSync(["gh", ...args], {
    cwd: ROOT,
    stdout: "inherit",
    stderr: "inherit",
  });
  return proc.exitCode ?? 1;
}

function loadRelease(tag: string) {
  try {
    return parseReleaseView(ghJson(["release", "view", tag, "--json", "isDraft,assets,url"]));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (/release not found/i.test(message) || /not found/i.test(message)) return null;
    throw error;
  }
}

function draftVersion(): string {
  return versionFromCheckout(process.argv[3]);
}

function signDraft(): void {
  if (process.platform !== "win32") {
    throw new Error(
      "Signing runs only on the maintainer's Windows PC, with SimplySign Desktop signed in. Run `just sign-draft vX.Y.Z` there.",
    );
  }
  const version = draftVersion();
  const tag = tagForVersion(version);
  runCmd(["pwsh", "-NoLogo", "-NoProfile", "-File", join(ROOT, "scripts", "sign-windows.ps1"), "-Check"]);
  ensureGh();
  const view = loadRelease(tag);
  assertUnsignedDraft(view, tag);

  const dir = join(ROOT, "target", "draft-sign");
  mkdirSync(dir, { recursive: true });
  const downloaded = ghInherit([
    "release",
    "download",
    tag,
    "--dir",
    dir,
    "--clobber",
    "--pattern",
    UNSIGNED_EXE,
    "--pattern",
    `${UNSIGNED_EXE}.sha256`,
  ]);
  if (downloaded !== 0) {
    throw new Error(`Could not download ${UNSIGNED_EXE} from draft ${tag}.`);
  }
  const exePath = join(dir, UNSIGNED_EXE);
  const bytes = readFileSync(exePath);
  assertChecksum(bytes, readFileSync(`${exePath}.sha256`, "utf8"), UNSIGNED_EXE);
  assertWindowsExecutable(bytes);

  mkdirSync(DIST_DIR, { recursive: true });
  try {
    copyFileSync(exePath, DIST_BIN);
  } catch (error) {
    const code = (error as NodeJS.ErrnoException).code;
    if (code === "EBUSY" || code === "EPERM") {
      throw new Error(`${DIST_BIN} is in use. Close Lightcal, then run just sign-draft ${tag} again.`);
    }
    throw error;
  }
  process.env.LIGHTCAL_RELEASE_VERSION = version;
  signWithSigntool(DIST_BIN);
  verifySignature(DIST_BIN);
  packagePortableArchive();
  signInstaller();

  const uploads = signedUploadPaths(ROOT, version);
  assertFilesExist(uploads, existsSync);
  console.log(`=== Uploading signed files to draft ${tag} ===`);
  if (ghInherit(uploadArgs(tag, uploads)) !== 0) {
    throw new Error(`Upload to draft ${tag} failed. Re-run just sign-draft ${tag} after signing finishes.`);
  }
  console.log(`Uploaded. The release stays a draft. Publish it with: just publish-release ${tag}`);
}

function publishDraft(): void {
  const version = draftVersion();
  const tag = tagForVersion(version);
  ensureGh();
  const view = loadRelease(tag);
  assertUnsignedDraft(view, tag);
  const missing = missingSignedAssets(view!, version);
  if (missing.length > 0) {
    throw new Error(
      `Draft ${tag} is missing signed files: ${missing.join(", ")}. On Windows, run just sign-draft ${tag}.`,
    );
  }
  for (const name of unsignedAssetsPresent(view!, tag)) {
    console.log(`Removing unsigned draft file ${name}`);
    if (ghInherit(deleteAssetArgs(tag, name)) !== 0) {
      throw new Error(`Could not remove unsigned file ${name} from ${tag}.`);
    }
  }
  const notesDir = join(ROOT, "target", "draft-sign");
  mkdirSync(notesDir, { recursive: true });
  const notesPath = join(notesDir, "published-notes.md");
  writeFileSync(notesPath, publishedReleaseNotes(version), "utf8");
  console.log(`=== Publishing ${tag} ===`);
  if (ghInherit(editReleaseArgs(tag, publishedReleaseTitle(version), notesPath)) !== 0) {
    throw new Error(`Could not publish ${tag}. It remains a draft when the edit fails.`);
  }
  console.log(`Published ${tag}.`);
}

function runDraftCommand(action: "sign-draft" | "publish-release"): void {
  try {
    if (action === "sign-draft") signDraft();
    else publishDraft();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}

const action = (process.argv[2] || "release").toLowerCase();
switch (action) {
  case "build":
    buildRelease();
    writeChecksums(DIST_BIN);
    break;
  case "sign":
    signWithSigntool();
    verifySignature();
    break;
  case "verify":
    verifySignature();
    break;
  case "certificate-info":
    showCertificateFingerprint();
    break;
  case "package":
    packagePortableArchive();
    break;
  case "sign-installer":
    signInstaller();
    break;
  case "release":
    buildRelease();
    signWithSigntool();
    verifySignature();
    packagePortableArchive();
    signInstaller();
    break;
  case "sign-draft":
    runDraftCommand("sign-draft");
    break;
  case "publish-release":
    runDraftCommand("publish-release");
    break;
  default:
    console.error(
      `Unknown command '${action}'. Expected: build | sign | verify | certificate-info | package | sign-installer | release | sign-draft | publish-release`
    );
    process.exit(1);
}
