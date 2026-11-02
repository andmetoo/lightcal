/**
 * Plan the Windows release signing flow.
 *
 * CI attaches an unsigned Windows build. signtool can sign only on the maintainer's
 * PC while SimplySign Desktop is signed in. `just sign-draft vX.Y.Z` downloads that
 * release (draft or public); `just publish-release vX.Y.Z` explicitly promotes it.
 */
import { createHash } from "node:crypto";
import { join } from "node:path";

export const UNSIGNED_EXE = "lightcal-unsigned.exe";
export const SIGNED_EXE = "lightcal.exe";

const STABLE_VERSION = /^v?([0-9]+\.[0-9]+\.[0-9]+)$/;

export type ReleaseAsset = { name: string };
export type ReleaseView = { isDraft: boolean; url?: string; assets: ReleaseAsset[] };
export type BuildRun = { databaseId: number; headSha: string; status: string; event: string };

export function matchingBuildRuns(runs: BuildRun[], commit: string): BuildRun[] {
  // A release-upload failure does not invalidate an already uploaded build artifact.
  return runs.filter((run) => run.headSha === commit && run.status === "completed" && run.event === "push");
}

/** Returns X.Y.Z from "X.Y.Z" or "vX.Y.Z", or null for anything else. */
export function parseStableVersion(raw: string | undefined): string | null {
  return raw?.trim().match(STABLE_VERSION)?.[1] ?? null;
}

export function requireStableVersion(raw: string | undefined): string {
  const version = parseStableVersion(raw);
  if (!version) {
    throw new Error(`Expected a stable version such as v0.2.0${raw?.trim() ? `: ${raw.trim()}` : ""}`);
  }
  return version;
}

export function sha256Hex(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

export function versionFromCheckout(explicit: string | undefined): string {
  if (!explicit?.trim()) {
    throw new Error("Pass the draft version: just sign-draft vX.Y.Z");
  }
  return requireStableVersion(explicit);
}

export function tagForVersion(version: string): string {
  return `v${version}`;
}

export function unsignedZipName(tag: string): string {
  return `lightcal-${tag}-windows-x64-unsigned.zip`;
}

export function unsignedSetupName(tag: string): string {
  return `lightcal-${tag}-windows-x64-unsigned-setup.exe`;
}

export function signedZipName(version: string): string {
  return `lightcal-${tagForVersion(version)}-windows-x64.zip`;
}

export function signedSetupName(version: string): string {
  return `lightcal-${version}-windows-x64-setup.exe`;
}

export function unsignedAssetNames(tag: string): string[] {
  const zip = unsignedZipName(tag);
  const setup = unsignedSetupName(tag);
  return [UNSIGNED_EXE, `${UNSIGNED_EXE}.sha256`, zip, `${zip}.sha256`, setup, `${setup}.sha256`];
}

export function signedAssetNames(version: string): string[] {
  const zip = signedZipName(version);
  const setup = signedSetupName(version);
  return [SIGNED_EXE, `${SIGNED_EXE}.sha256`, zip, `${zip}.sha256`, setup, `${setup}.sha256`];
}

export function signedUploadPaths(root: string, version: string): string[] {
  return [
    join(root, "target", "dist", SIGNED_EXE),
    join(root, "target", "dist", `${SIGNED_EXE}.sha256`),
    join(root, "target", "dist", signedZipName(version)),
    join(root, "target", "dist", `${signedZipName(version)}.sha256`),
    join(root, "target", "nsis", signedSetupName(version)),
    join(root, "target", "nsis", `${signedSetupName(version)}.sha256`),
  ];
}

export function assertFilesExist(paths: string[], exists: (path: string) => boolean): void {
  const missing = paths.filter((path) => !exists(path));
  if (missing.length > 0) {
    throw new Error(`Missing signed file: ${missing.join(", ")}`);
  }
}

export function parseReleaseView(json: string): ReleaseView {
  let value: unknown;
  try {
    value = JSON.parse(json);
  } catch {
    throw new Error("GitHub release metadata is not JSON");
  }
  if (!value || typeof value !== "object") throw new Error("GitHub release metadata is incomplete");
  const record = value as { isDraft?: unknown; assets?: unknown; url?: unknown };
  if (typeof record.isDraft !== "boolean" || !Array.isArray(record.assets)) {
    throw new Error("GitHub release metadata is incomplete");
  }
  const assets = record.assets.map((asset) => {
    if (!asset || typeof asset !== "object" || typeof (asset as { name?: unknown }).name !== "string") {
      throw new Error("GitHub release asset is missing a name");
    }
    return { name: (asset as { name: string }).name };
  });
  return {
    isDraft: record.isDraft,
    url: typeof record.url === "string" ? record.url : undefined,
    assets,
  };
}

export function assertReleaseExists(view: ReleaseView | null, tag: string): asserts view is ReleaseView {
  if (!view) {
    throw new Error(
      `No GitHub release ${tag} yet. Check the Windows workflow in GitHub Actions. ` +
      `To build an existing tag again with the current workflow: gh workflow run windows-build.yml --ref main -f release_tag=${tag}`,
    );
  }
}

export function signingAsset(view: ReleaseView): string {
  const names = new Set(view.assets.map((asset) => asset.name));
  // Prefer the ordinary name, but keep older releases signable.
  for (const name of [SIGNED_EXE, UNSIGNED_EXE]) {
    if (names.has(name) && names.has(`${name}.sha256`)) return name;
  }
  throw new Error(`Release needs lightcal.exe and lightcal.exe.sha256 (legacy lightcal-unsigned.exe is also supported). Found: ${[...names].join(", ") || "no assets"}`);
}

/** Names alone cannot prove signing: CI and the signer use the same names. */
export function verifyReleaseFiles(
  directory: string,
  version: string,
  read: (path: string) => Uint8Array,
  verifyExecutable: (path: string) => void,
  verifyArchive: (path: string, executable: string) => void,
): void {
  for (const name of [SIGNED_EXE, signedZipName(version), signedSetupName(version)]) {
    assertChecksum(read(join(directory, name)), new TextDecoder().decode(read(join(directory, `${name}.sha256`))), name);
  }
  const executable = join(directory, SIGNED_EXE);
  verifyExecutable(executable);
  verifyExecutable(join(directory, signedSetupName(version)));
  verifyArchive(join(directory, signedZipName(version)), executable);
}

export function missingSignedAssets(view: ReleaseView, version: string): string[] {
  const names = new Set(view.assets.map((asset) => asset.name));
  return signedAssetNames(version).filter((name) => !names.has(name));
}

export function unsignedAssetsPresent(view: ReleaseView, tag: string): string[] {
  const names = new Set(view.assets.map((asset) => asset.name));
  return unsignedAssetNames(tag).filter((name) => names.has(name));
}

export function parseChecksum(text: string): { hash: string; name: string } {
  const line = text
    .split(/\r?\n/)
    .map((item) => item.trim())
    .find((item) => item && !item.startsWith("#"));
  if (!line) throw new Error("Checksum file is empty");
  const match = line.match(/^([0-9a-fA-F]{64})\s+\*?(\S+)$/);
  if (!match) throw new Error(`Checksum line is not SHA-256: ${line}`);
  return { hash: match[1].toLowerCase(), name: match[2] };
}

export function assertChecksum(bytes: Uint8Array, checksumText: string, fileName: string): string {
  const parsed = parseChecksum(checksumText);
  if (parsed.name !== fileName) {
    throw new Error(`Checksum names ${parsed.name}, expected ${fileName}`);
  }
  const actual = sha256Hex(bytes);
  if (actual !== parsed.hash) throw new Error(`SHA-256 mismatch for ${fileName}`);
  return actual;
}

export function assertWindowsExecutable(bytes: Uint8Array): void {
  if (bytes.length < 2 || bytes[0] !== 0x4d || bytes[1] !== 0x5a) {
    throw new Error("Draft file is not a Windows executable");
  }
}

export function uploadArgs(tag: string, files: string[]): string[] {
  return ["release", "upload", tag, ...files, "--clobber"];
}

export function deleteAssetArgs(tag: string, name: string): string[] {
  return ["release", "delete-asset", tag, name, "--yes"];
}

export function editReleaseArgs(tag: string, title: string, notesFile: string): string[] {
  return [
    "release",
    "edit",
    tag,
    "--draft=false",
    "--prerelease=false",
    "--latest",
    "--title",
    title,
    "--notes-file",
    notesFile,
  ];
}

export function publishedReleaseTitle(version: string): string {
  return `Lightcal ${tagForVersion(version)}`;
}

export function publishedReleaseNotes(version: string): string {
  const tag = tagForVersion(version);
  return [
    `Lightcal ${tag}.`,
    "",
    "Downloads:",
    "",
    `- \`lightcal-${version}-windows-x64-setup.exe\`: installer (recommended).`,
    `- \`lightcal-${tag}-windows-x64.zip\`: portable version, no installation needed.`,
    "- `lightcal.exe`: the app on its own. Lightcal's automatic updates install this file.",
    "",
    "Every file is digitally signed, and each has a `.sha256` checksum next to it.",
    "",
  ].join("\n");
}

export function normalizeThumbprint(raw: string): string {
  return raw.replace(/\s+/g, "").toUpperCase();
}

/** Compares a signer thumbprint with the pinned SIGN_CERT_SHA1 (case-insensitive, spaces ignored). */
export function thumbprintMatches(expected: string | undefined, actual: string): boolean {
  const pin = normalizeThumbprint(expected ?? "");
  return pin.length > 0 && pin === normalizeThumbprint(actual);
}

/**
 * Whether an executable embeds `certSha256` as its update pin. The app compiles
 * LIGHTCAL_UPDATE_CERT_SHA256 in verbatim, so the fingerprint appears as ASCII hex in either
 * case, optionally colon-separated, and possibly inside a comma-separated list.
 */
export function embedsCertificatePin(executable: Uint8Array, certSha256: string): boolean {
  const hex = certSha256.replace(/[\s:]/g, "").toLowerCase();
  if (!/^[0-9a-f]{64}$/.test(hex)) throw new Error(`Not a SHA-256 certificate fingerprint: ${certSha256}`);
  const colons = hex.match(/../g)!.join(":");
  const text = Buffer.from(executable).toString("latin1");
  return [hex, hex.toUpperCase(), colons, colons.toUpperCase()].some((form) => text.includes(form));
}
