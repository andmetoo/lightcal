#!/usr/bin/env bun
/**
 * Lightcal release tooling: build, sign with SimplySign Desktop, package and publish.
 *
 *   bun run scripts/release.ts build | sign | verify | certificate-info | package | sign-installer | release
 *   bun run scripts/release.ts sign-draft vX.Y.Z       # sign a CI build and upload it to its release
 *   bun run scripts/release.ts publish-release vX.Y.Z  # verify the signed assets, then publish
 *
 * Signing uses scripts/sign-windows.ps1 with SIGN_CERT_SHA1 (a thumbprint, not a secret)
 * from .env; SimplySign Desktop must be signed in. SIGNTOOL overrides the signtool path.
 */
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import {
  assertChecksum,
  assertFilesExist,
  assertReleaseExists,
  assertWindowsExecutable,
  deleteAssetArgs,
  editReleaseArgs,
  matchingBuildRuns,
  missingSignedAssets,
  parseReleaseView,
  parseStableVersion,
  publishedReleaseNotes,
  publishedReleaseTitle,
  sha256Hex,
  signedUploadPaths,
  signingAsset,
  tagForVersion,
  thumbprintMatches,
  unsignedAssetsPresent,
  uploadArgs,
  verifyReleaseFiles,
  versionFromCheckout,
} from "./release-draft.ts";
import { ensureGh as ensureGhAt } from "./gh.ts";

const ROOT = resolve(import.meta.dir, "..");
const SCRIPTS = join(ROOT, "scripts");
const BIN_NAME = process.platform === "win32" ? "lightcal.exe" : "lightcal";
const RELEASE_BIN = join(ROOT, "target", "release", BIN_NAME);
const DIST_DIR = join(ROOT, "target", "dist");
const DIST_BIN = join(DIST_DIR, BIN_NAME);

if (existsSync(join(ROOT, ".env"))) process.loadEnvFile(join(ROOT, ".env"));

// --- process helpers --------------------------------------------------------

const pwsh = (...args: string[]) => ["pwsh", "-NoLogo", "-NoProfile", ...args];
const psQuote = (value: string) => `'${value.replaceAll("'", "''")}'`;

/** Runs a command with inherited output; throws `failure` (or a generic message) on a non-zero exit. */
function run(cmd: string[], failure?: string): void {
  console.log(`> ${cmd.join(" ")}`);
  const { exitCode } = Bun.spawnSync(cmd, { cwd: ROOT, stdout: "inherit", stderr: "inherit" });
  if (exitCode !== 0) throw new Error(failure ?? `${basename(cmd[0])} exited with code ${exitCode}`);
}

/** Runs a command and returns its trimmed stdout; stderr becomes the error message. */
function output(cmd: string[]): string {
  const proc = Bun.spawnSync(cmd, { cwd: ROOT, stdout: "pipe", stderr: "pipe" });
  if (proc.exitCode !== 0) throw new Error(proc.stderr.toString().trim() || `${cmd.join(" ")} failed`);
  return proc.stdout.toString().trim();
}

const gh = (...args: string[]) => output(["gh", ...args]);
const ghRun = (args: string[], failure: string) => run(["gh", ...args], failure);

const ensureGh = () => ensureGhAt(ROOT);

function requireWindows(message: string): void {
  if (process.platform !== "win32") throw new Error(message);
}

// --- build, sign, verify, package ------------------------------------------

let resolvedVersion: string | undefined;

/** Resolves once per process; sign-draft sets LIGHTCAL_RELEASE_VERSION before the first call. */
function releaseVersion(): string {
  resolvedVersion ??= resolveReleaseVersion();
  return resolvedVersion;
}

function resolveReleaseVersion(): string {
  const sources = [
    () => process.env.LIGHTCAL_RELEASE_VERSION,
    () => (process.env.GITHUB_REF_TYPE === "tag" ? process.env.GITHUB_REF_NAME : undefined),
    () => {
      const described = Bun.spawnSync(["git", "describe", "--tags", "--exact-match"], { cwd: ROOT, stderr: "ignore" });
      return described.exitCode === 0 ? described.stdout.toString() : undefined;
    },
    () => readFileSync(join(ROOT, "Cargo.toml"), "utf8").match(/^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"/m)?.[1],
  ];
  for (const source of sources) {
    const version = parseStableVersion(source());
    if (version) return version;
  }
  throw new Error("Workspace version is missing or invalid");
}

function stageReleaseBinary(): void {
  if (!existsSync(RELEASE_BIN)) throw new Error(`Expected release binary not found at ${RELEASE_BIN}`);
  mkdirSync(DIST_DIR, { recursive: true });
  copyFileSync(RELEASE_BIN, DIST_BIN);
}

function buildRelease(): void {
  console.log("=== Building Lightcal release binary ===");
  run(["cargo", "build", "--release", "--workspace", "--locked"]);
  stageReleaseBinary();
}

/** Writes `<file>.sha256` in `sha256sum` format and prints size and hash. */
function writeChecksum(path: string): void {
  const bytes = readFileSync(path);
  const hash = sha256Hex(bytes);
  writeFileSync(`${path}.sha256`, `${hash}  ${basename(path)}\n`, "utf8");
  console.log(`${path}\n  ${(bytes.byteLength / 1048576).toFixed(2)} MiB (${bytes.byteLength} bytes), SHA-256 ${hash}`);
}

function sign(target = DIST_BIN): void {
  requireWindows("signtool can sign only on the maintainer's Windows PC. Run `just sign-draft` there.");
  if (target === DIST_BIN && !existsSync(DIST_BIN)) {
    if (existsSync(RELEASE_BIN)) stageReleaseBinary();
    else buildRelease();
  }
  if (!existsSync(target)) throw new Error(`Missing signing target: ${target}`);
  console.log(`=== Signing ${target} with signtool ===`);
  run(pwsh("-File", join(SCRIPTS, "sign-windows.ps1"), "-File", target));
}

/** signtool from SIGNTOOL/SIGNTOOL_PATH, the newest Windows 10/11 SDK, or PATH. */
function findSignTool(): string | null {
  const configured = process.env.SIGNTOOL || process.env.SIGNTOOL_PATH;
  if (configured && existsSync(configured)) return configured;
  const kits = "C:\\Program Files (x86)\\Windows Kits\\10\\bin";
  if (existsSync(kits)) {
    const parts = (name: string) => name.split(".").map(Number);
    const versions = readdirSync(kits).filter((name) => /^10(\.\d+)+$/.test(name)).sort((a, b) => {
      const [x, y] = [parts(a), parts(b)];
      for (let i = 0; i < Math.max(x.length, y.length); i++) if ((x[i] ?? 0) !== (y[i] ?? 0)) return (y[i] ?? 0) - (x[i] ?? 0);
      return 0;
    });
    for (const version of versions) {
      const candidate = join(kits, version, "x64", "signtool.exe");
      if (existsSync(candidate)) return candidate;
    }
  }
  return Bun.which("signtool");
}

type Signature = { status: string; message: string; thumbprint: string | null; subject: string | null };

/**
 * Requires a valid Authenticode signature: `signtool verify /pa` when the SDK is present, and
 * PowerShell's Get-AuthenticodeSignature always. With SIGN_CERT_SHA1 set, the signer must match it.
 * `writeSum` refreshes the local `.sha256`; publication passes false to leave downloaded files untouched.
 */
function verify(target = existsSync(DIST_BIN) ? DIST_BIN : RELEASE_BIN, requirePin = false, writeSum = true): void {
  const pin = process.env.SIGN_CERT_SHA1?.trim();
  if (requirePin && !pin) {
    throw new Error("SIGN_CERT_SHA1 is not set. Put the code-signing thumbprint in .env so publication can verify the signer.");
  }
  if (!existsSync(target)) throw new Error(`No binary found at ${target}. Run 'just release build' first.`);
  console.log(`=== Verifying Authenticode signature on ${target} ===`);
  const signtool = findSignTool();
  if (signtool) run([signtool, "verify", "/pa", "/v", target], `signtool rejected the signature of ${target}.`);
  // One compact JSON object on the last stdout line, so warnings or unusual subjects cannot shift fields.
  const lines = output(pwsh("-Command",
    `$sig = Get-AuthenticodeSignature -LiteralPath ${psQuote(target)}; ` +
    `[ordered]@{ status = "$($sig.Status)"; message = $sig.StatusMessage; ` +
    `thumbprint = $sig.SignerCertificate.Thumbprint; subject = $sig.SignerCertificate.Subject } | ConvertTo-Json -Compress`,
  )).split(/\r?\n/);
  const signature = JSON.parse(lines[lines.length - 1]) as Signature;
  if (signature.status !== "Valid") throw new Error(`Signature of ${target} is ${signature.status}: ${signature.message}`);
  if (pin && !thumbprintMatches(pin, signature.thumbprint ?? "")) {
    throw new Error(`Signer certificate of ${target} does not match SIGN_CERT_SHA1.`);
  }
  console.log(`Valid signature by ${signature.subject} (${signature.thumbprint})`);
  if (writeSum) writeChecksum(target);
}

function showCertificateFingerprint(): void {
  const target = existsSync(DIST_BIN) ? DIST_BIN : RELEASE_BIN;
  verify(target);
  run(pwsh("-Command",
    `$sig = Get-AuthenticodeSignature -LiteralPath ${psQuote(target)}; ` +
    `[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($sig.SignerCertificate.RawData))`,
  ));
}

function packagePortableArchive(): void {
  if (!existsSync(DIST_BIN)) buildRelease();
  writeChecksum(DIST_BIN);

  const name = `lightcal-v${releaseVersion()}-windows-x64`;
  const stage = join(DIST_DIR, name);
  mkdirSync(stage, { recursive: true });
  for (const file of [DIST_BIN, `${DIST_BIN}.sha256`]) copyFileSync(file, join(stage, basename(file)));
  for (const doc of ["LICENSE", "THIRD_PARTY_NOTICES.md", "README.md"]) {
    if (existsSync(join(ROOT, doc))) copyFileSync(join(ROOT, doc), join(stage, doc));
  }

  const zip = join(DIST_DIR, `${name}.zip`);
  if (process.platform === "win32") {
    run(pwsh("-Command", `Compress-Archive -Path ${psQuote(`${stage}\\*`)} -DestinationPath ${psQuote(zip)} -Force`));
  }
  if (existsSync(zip)) writeChecksum(zip);
}

function signInstaller(): void {
  verify(DIST_BIN);
  const version = releaseVersion();
  run(pwsh("-File", join(SCRIPTS, "package-nsis.ps1"), "-SkipBuild", "-RequireSigned", "-BinaryPath", DIST_BIN, "-Version", version));
  const setup = join(ROOT, "target", "nsis", `lightcal-${version}-windows-x64-setup.exe`);
  sign(setup);
  verify(setup);
}

function assertExecutableVersion(path: string, version: string): void {
  run(pwsh("-Command",
    `$actual = [Diagnostics.FileVersionInfo]::GetVersionInfo(${psQuote(path)}).ProductVersion; ` +
    `if ($actual -ne '${version}') { throw "Payload ProductVersion $actual differs from release version ${version}" }`,
  ));
}

// --- GitHub draft signing and publication -----------------------------------

function loadRelease(tag: string) {
  try {
    return parseReleaseView(gh("release", "view", tag, "--json", "isDraft,assets,url"));
  } catch (error) {
    if (/release not found/i.test(String(error))) return null;
    throw error;
  }
}

function releaseDownloadDirectory(tag: string): string {
  const parent = join(ROOT, "target", "release-sign", tag);
  mkdirSync(parent, { recursive: true });
  return mkdtempSync(join(parent, "download-"));
}

/** Downloads the executable to sign: the release asset if present, else the matching CI artifact. */
function downloadSigningInput(view: ReturnType<typeof loadRelease>, tag: string, dir: string): string {
  let asset: string | undefined;
  try { asset = view ? signingAsset(view) : undefined; } catch { /* fall back to the CI artifact */ }
  if (asset) {
    ghRun(["release", "download", tag, "--dir", dir, "--pattern", asset, "--pattern", `${asset}.sha256`],
      `Could not download ${asset} from release ${tag}.`);
    return asset;
  }
  const artifact = "lightcal-windows-x64-unsigned";
  const commit = gh("api", `repos/{owner}/{repo}/commits/${tag}`, "--jq", ".sha");
  const runs = JSON.parse(gh("run", "list", "--workflow", "windows-build.yml", "--branch", tag,
    "--event", "push", "--limit", "30", "--json", "databaseId,headSha,status,event"));
  for (const build of matchingBuildRuns(runs, commit)) {
    const { artifacts } = JSON.parse(gh("api", `repos/{owner}/{repo}/actions/runs/${build.databaseId}/artifacts`));
    if (!artifacts.some((item: { name: string; expired: boolean }) => item.name === artifact && !item.expired)) continue;
    console.log(`Release has no application asset. Downloading CI build ${build.databaseId} for ${tag} (${commit}).`);
    ghRun(["run", "download", String(build.databaseId), "--name", artifact, "--dir", dir],
      `Could not download build ${build.databaseId}.`);
    return signingAsset({ isDraft: true, assets: readdirSync(dir).map((name) => ({ name })) });
  }
  throw new Error(`No completed Windows build artifact for ${tag} (${commit}). Wait for the build in GitHub Actions. ` +
    `To rebuild an existing tag: gh workflow run windows-build.yml --ref main -f release_tag=${tag}`);
}

function signDraft(): void {
  requireWindows("Signing runs only on the maintainer's Windows PC, with SimplySign Desktop signed in. Run `just sign-draft vX.Y.Z` there.");
  const version = versionFromCheckout(process.argv[3]);
  const tag = tagForVersion(version);
  ensureGh();
  const view = loadRelease(tag);
  run(pwsh("-File", join(SCRIPTS, "sign-windows.ps1"), "-Check"));

  const dir = releaseDownloadDirectory(tag);
  const asset = downloadSigningInput(view, tag, dir);
  const exePath = join(dir, asset);
  const bytes = readFileSync(exePath);
  assertChecksum(bytes, readFileSync(`${exePath}.sha256`, "utf8"), asset);
  assertWindowsExecutable(bytes);
  assertExecutableVersion(exePath, version);

  mkdirSync(DIST_DIR, { recursive: true });
  try {
    copyFileSync(exePath, DIST_BIN);
  } catch (error) {
    const code = (error as NodeJS.ErrnoException).code;
    if (code === "EBUSY" || code === "EPERM") throw new Error(`${DIST_BIN} is in use. Close Lightcal, then run just sign-draft ${tag} again.`);
    throw error;
  }
  process.env.LIGHTCAL_RELEASE_VERSION = version;
  sign(DIST_BIN);
  verify(DIST_BIN);
  packagePortableArchive();
  signInstaller();

  const uploads = signedUploadPaths(ROOT, version);
  assertFilesExist(uploads, existsSync);
  // Only an explicit publish-release changes visibility.
  if (!loadRelease(tag)) {
    ghRun(["release", "create", tag, "--verify-tag", "--draft", "--title", publishedReleaseTitle(version),
      "--notes", "Signed Windows build. Awaiting explicit publication."],
      `Could not create draft ${tag}; signed files remain in target/dist and target/nsis.`);
  }
  console.log(`=== Uploading signed files to release ${tag} ===`);
  ghRun(uploadArgs(tag, uploads), `Upload to release ${tag} failed. Re-run just sign-draft ${tag} after signing finishes.`);
  console.log(`Uploaded. Release visibility is unchanged. To publish/promote it: just publish-release ${tag}`);
}

function publishDraft(): void {
  requireWindows("Publish from Windows so Authenticode can be verified before publication.");
  const version = versionFromCheckout(process.argv[3]);
  const tag = tagForVersion(version);
  ensureGh();
  const view = loadRelease(tag);
  assertReleaseExists(view, tag);
  const missing = missingSignedAssets(view, version);
  if (missing.length > 0) throw new Error(`Release ${tag} is missing files: ${missing.join(", ")}. On Windows, run just sign-draft ${tag}.`);

  const downloaded = releaseDownloadDirectory(tag);
  const names = signedUploadPaths(ROOT, version).map((path) => basename(path));
  ghRun(["release", "download", tag, "--dir", downloaded, ...names.flatMap((name) => ["--pattern", name])],
    `Could not download release ${tag} for signature verification.`);
  verifyReleaseFiles(downloaded, version, readFileSync, (path) => verify(path, true, false), (archive, executable) =>
    run(pwsh("-File", join(SCRIPTS, "verify-portable.ps1"), "-Archive", archive, "-Executable", executable)));
  assertExecutableVersion(join(downloaded, "lightcal.exe"), version);
  assertExecutableVersion(join(downloaded, `lightcal-${version}-windows-x64-setup.exe`), version);

  for (const name of unsignedAssetsPresent(view, tag)) {
    console.log(`Removing unsigned draft file ${name}`);
    ghRun(deleteAssetArgs(tag, name), `Could not remove unsigned file ${name} from ${tag}.`);
  }
  const notesDir = join(ROOT, "target", "draft-sign");
  mkdirSync(notesDir, { recursive: true });
  const notesPath = join(notesDir, "published-notes.md");
  writeFileSync(notesPath, publishedReleaseNotes(version), "utf8");
  console.log(`=== Publishing ${tag} ===`);
  ghRun(editReleaseArgs(tag, publishedReleaseTitle(version), notesPath),
    `Could not publish ${tag}. Check the release state in GitHub before retrying.`);
  console.log(`Published ${tag}.`);
}

// --- entry point -------------------------------------------------------------

const commands: Record<string, () => void> = {
  build: () => { buildRelease(); writeChecksum(DIST_BIN); },
  sign: () => { sign(); verify(); },
  verify: () => verify(),
  "certificate-info": showCertificateFingerprint,
  package: packagePortableArchive,
  "sign-installer": signInstaller,
  release: () => { buildRelease(); sign(); verify(); packagePortableArchive(); signInstaller(); },
  "sign-draft": signDraft,
  "publish-release": publishDraft,
};

const action = (process.argv[2] || "release").toLowerCase();
const command = commands[action];
if (!command) {
  console.error(`Unknown command '${action}'. Expected: ${Object.keys(commands).join(" | ")}`);
  process.exit(1);
}
try {
  command();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
