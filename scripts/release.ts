#!/usr/bin/env bun
/**
 * Lightcal release build, SimpleSign Desktop signing workflow, and SHA-256 checksum runner.
 *
 * Commands (via Bun or `just`):
 *   bun run scripts/release.ts build    # Build optimized release binary, stage to target/dist/, write SHA-256
 *   bun run scripts/release.ts sign     # Sign binary via SimpleSign Desktop, verify signature, update SHA-256
 *   bun run scripts/release.ts verify   # Verify Authenticode signature & update SHA-256 after SimpleSign Desktop
 *   bun run scripts/release.ts release  # Full pipeline: build -> stage -> SimpleSign Desktop -> checksum
 *
 * Optional environment variables:
 *   SIMPLESIGN_PATH  - Path to SimpleSign Desktop executable (auto-detected if installed in standard paths)
 *   SIMPLESIGN_ARGS  - Extra CLI flags for SimpleSign Desktop (default passes the target binary path)
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
import { join, resolve } from "node:path";

const ROOT = resolve(import.meta.dir, "..");
const BIN_NAME = process.platform === "win32" ? "lightcal.exe" : "lightcal";
const RELEASE_BIN = join(ROOT, "target", "release", BIN_NAME);
const DIST_DIR = join(ROOT, "target", "dist");
const DIST_BIN = join(DIST_DIR, BIN_NAME);

function runCmd(cmd: string[], cwd = ROOT, allowFailure = false): number {
  console.log(`> ${cmd.join(" ")}`);
  const proc = Bun.spawnSync(cmd, {
    cwd,
    stdout: "inherit",
    stderr: "inherit",
  });
  if (proc.exitCode !== 0 && !allowFailure) {
    process.exit(proc.exitCode ?? 1);
  }
  return proc.exitCode ?? 0;
}

function findSimpleSignDesktop(): string | null {
  if (process.env.SIMPLESIGN_PATH && existsSync(process.env.SIMPLESIGN_PATH)) {
    return process.env.SIMPLESIGN_PATH;
  }

  const localAppData = process.env.LOCALAPPDATA || "";
  const programFiles = process.env.ProgramFiles || "C:\\Program Files";
  const userProfile = process.env.USERPROFILE || "";

  const candidates = [
    join(localAppData, "Programs", "SimpleSign Desktop", "SimpleSign Desktop.exe"),
    join(localAppData, "Programs", "simplesign-desktop", "SimpleSign Desktop.exe"),
    join(localAppData, "SimpleSignDesktop", "SimpleSign Desktop.exe"),
    join(programFiles, "SimpleSign Desktop", "SimpleSign Desktop.exe"),
    join(userProfile, "Desktop", "SimpleSign Desktop.exe"),
  ];

  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }

  const which = Bun.which("simplesign-desktop") || Bun.which("simplesign");
  return which ?? null;
}

function findSignTool(): string | null {
  if (process.env.SIGNTOOL_PATH && existsSync(process.env.SIGNTOOL_PATH)) {
    return process.env.SIGNTOOL_PATH;
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

  const checksumLine = `${sha256}  ${BIN_NAME}\n`;
  writeFileSync(`${targetPath}.sha256`, checksumLine, "utf8");

  console.log(`Staged binary: ${targetPath}`);
  console.log(`Payload size:  ${sizeMiB} MiB (${bytes.byteLength} bytes)`);
  console.log(`SHA-256:       ${sha256}`);
}

function signWithSimpleSign(): void {
  if (!existsSync(DIST_BIN)) {
    if (existsSync(RELEASE_BIN)) {
      mkdirSync(DIST_DIR, { recursive: true });
      copyFileSync(RELEASE_BIN, DIST_BIN);
    } else {
      buildRelease();
    }
  }

  const simpleSignExe = findSimpleSignDesktop();
  const extraArgs = process.env.SIMPLESIGN_ARGS
    ? process.env.SIMPLESIGN_ARGS.split(/\s+/).filter(Boolean)
    : [];

  if (simpleSignExe) {
    console.log(`=== Launching SimpleSign Desktop for ${DIST_BIN} ===`);
    runCmd([simpleSignExe, ...extraArgs, DIST_BIN]);
  } else {
    throw new Error(`SimpleSign Desktop was not found. Sign ${DIST_BIN} manually, then run 'just verify-release' and 'just package-release'.`);
  }
}

function verifySignature(): void {
  const target = existsSync(DIST_BIN) ? DIST_BIN : RELEASE_BIN;
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

  const manifest = readFileSync(join(ROOT, "Cargo.toml"), "utf8");
  const version = manifest.match(/^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"/m)?.[1];
  if (!version) throw new Error("Workspace version is missing or invalid");
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

const action = (process.argv[2] || "release").toLowerCase();
switch (action) {
  case "build":
    buildRelease();
    writeChecksums(DIST_BIN);
    break;
  case "sign":
    signWithSimpleSign();
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
  case "release":
    buildRelease();
    signWithSimpleSign();
    verifySignature();
    packagePortableArchive();
    break;
  default:
    console.error(
      `Unknown command '${action}'. Expected: build | sign | verify | certificate-info | package | release`
    );
    process.exit(1);
}
