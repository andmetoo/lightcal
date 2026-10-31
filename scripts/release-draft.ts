/**
 * Plan the Windows draft-release signing flow.
 *
 * CI opens an unsigned GitHub draft. signtool can sign only on the maintainer's
 * PC while SimplySign Desktop is signed in. `just sign-draft vX.Y.Z` downloads that
 * draft, and `just publish-release vX.Y.Z` is the explicit step that makes it public.
 */
import { createHash } from "node:crypto";
import { join } from "node:path";

export const UNSIGNED_EXE = "lightcal-unsigned.exe";
export const SIGNED_EXE = "lightcal.exe";

const STABLE_VERSION = /^v?([0-9]+\.[0-9]+\.[0-9]+)$/;

export type ReleaseAsset = { name: string };
export type ReleaseView = { isDraft: boolean; url?: string; assets: ReleaseAsset[] };

export function requireStableVersion(raw: string | undefined): string {
  const match = raw?.trim().match(STABLE_VERSION);
  if (!match) {
    throw new Error(`Expected a stable version such as v0.2.0${raw?.trim() ? `: ${raw.trim()}` : ""}`);
  }
  return match[1];
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

export function assertUnsignedDraft(view: ReleaseView | null, tag: string): void {
  if (!view) {
    throw new Error(
      `No GitHub release ${tag} yet. Wait until the Windows workflow opens the unsigned draft.`,
    );
  }
  if (!view.isDraft) {
    throw new Error(`Release ${tag} is already published. A new signed upload needs a new version.`);
  }
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
  const actual = createHash("sha256").update(bytes).digest("hex");
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
    "Signed on the maintainer's Windows PC with signtool while Certum SimplySign Desktop is signed in. Signing keys stay in that tool.",
    "Automatic installation uses the asset named exactly lightcal.exe.",
    `NSIS setup: lightcal-${version}-windows-x64-setup.exe.`,
    `Portable archive: lightcal-${tag}-windows-x64.zip.`,
    "",
  ].join("\n");
}
