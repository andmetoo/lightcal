/**
 * Plan the Windows release signing flow.
 *
 * CI attaches an unsigned Windows build. signtool can sign only on the maintainer's
 * PC while SimplySign Desktop is signed in. `just sign-draft vX.Y.Z` downloads that
 * release (draft or public); `just publish-release vX.Y.Z` explicitly promotes it.
 */
import { createHash, createPublicKey, verify as verifySignature } from "node:crypto";
import { join } from "node:path";
import { normalizeFingerprint } from "./certificate.ts";

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

/** Sparkle reads this asset from the latest stable release (see docs/macos.md). */
export const MACOS_APPCAST = "lightcal-macos-appcast.xml";

export function macZipName(version: string): string {
  return `lightcal-${version}-macos-universal.zip`;
}

export function macDmgName(version: string): string {
  return `lightcal-${version}-macos-universal.dmg`;
}

export function macAssetNames(version: string): string[] {
  const zip = macZipName(version);
  const dmg = macDmgName(version);
  return [zip, `${zip}.sha256`, dmg, `${dmg}.sha256`, MACOS_APPCAST];
}

/**
 * macOS files a release still needs before it may become the latest release.
 * Once a public release serves the macOS appcast, every later one must as well:
 * Sparkle's feed URL follows "latest", so a Windows-only release would break
 * updates for every signed Mac installation. Partial macOS files are also refused.
 */
export function missingMacAssets(
  target: ReleaseView,
  latest: ReleaseView | null,
  version: string,
  allowWindowsOnly = false,
): string[] {
  const names = new Set(target.assets.map((asset) => asset.name));
  const expected = macAssetNames(version);
  const started = expected.some((name) => names.has(name));
  // The override covers a release with no macOS files at all; partial files are always refused.
  if (!started && (allowWindowsOnly || !macShipped(latest))) return [];
  return expected.filter((name) => !names.has(name));
}

/** Whether a public release already serves the macOS appcast to signed Mac installs. */
export function macShipped(latest: ReleaseView | null): boolean {
  return latest !== null && !latest.isDraft && latest.assets.some((asset) => asset.name === MACOS_APPCAST);
}

/** Version of the macOS update ZIP attached to a release, or null when it has none. */
export function macZipVersion(view: ReleaseView | null): string | null {
  for (const asset of view?.assets ?? []) {
    const version = asset.name.match(/^lightcal-([0-9]+\.[0-9]+\.[0-9]+)-macos-universal\.zip$/)?.[1];
    if (version) return version;
  }
  return null;
}

/** Explicit exception for a Windows-only release after signed Mac releases (see docs/macos.md). */
export const ALLOW_WINDOWS_ONLY = "--allow-windows-only";

export function parsePublishOptions(args: string[]): { allowWindowsOnly: boolean } {
  const unknown = args.filter((arg) => arg !== ALLOW_WINDOWS_ONLY);
  if (unknown.length > 0) throw new Error(`Unknown publish-release option: ${unknown.join(" ")}. Supported: ${ALLOW_WINDOWS_ONLY}`);
  return { allowWindowsOnly: args.includes(ALLOW_WINDOWS_ONLY) };
}

/** Path of the bundle's Info.plist inside the `ditto --keepParent` update ZIP. */
export const MAC_INFO_PLIST = "Lightcal.app/Contents/Info.plist";

/** The Sparkle public key a Lightcal bundle trusts for its next update. */
export function sparkleKeyFromInfoPlist(plist: string): string {
  const key = plist.match(/<key>SUPublicEDKey<\/key>\s*<string>([^<]*)<\/string>/)?.[1]?.trim();
  if (!key) throw new Error(`${MAC_INFO_PLIST} has no SUPublicEDKey`);
  return key;
}

/**
 * Sparkle accepts an update only when it is signed for the key inside the installed
 * app, so the signing key may never change silently. `shipped` is the key of the
 * latest public Mac release (null before the first one); `next` is the key this
 * release embeds for its own successor.
 */
export function assertSparkleKeyContinuity(configured: string, next: string, shipped: string | null): void {
  if (shipped !== null && shipped !== configured) {
    throw new Error(
      `SPARKLE_PUBLIC_KEY differs from the key in the latest public Mac release (${shipped}). ` +
      "Installed Macs only accept updates signed for that key; restore the original key pair.",
    );
  }
  if (next !== configured) {
    throw new Error(`This release's app trusts SUPublicEDKey ${next}, not SPARKLE_PUBLIC_KEY; its own next update would be refused.`);
  }
}

/** Sparkle downloads the update from this exact URL, generated by scripts/package-macos.ts. */
export function expectedMacZipUrl(repository: string, version: string): string {
  return `https://github.com/${repository}/releases/download/${tagForVersion(version)}/${macZipName(version)}`;
}

export function hasMacAssets(view: ReleaseView, version: string): boolean {
  const names = new Set(view.assets.map((asset) => asset.name));
  return macAssetNames(version).every((name) => names.has(name));
}

export type AppcastEnclosure = { url: string; signature: string; length: number };

/** The single enclosure of a Lightcal appcast, after checking it announces `version`. */
export function parseAppcastEnclosure(appcast: string, version: string): AppcastEnclosure {
  if (!appcast.includes(`<sparkle:version>${version}</sparkle:version>`)) {
    throw new Error(`${MACOS_APPCAST} does not announce version ${version}`);
  }
  const enclosures = appcast.match(/<enclosure\s[^>]*>/g) ?? [];
  if (enclosures.length !== 1) throw new Error(`${MACOS_APPCAST} must contain exactly one enclosure`);
  const attribute = (name: string) => enclosures[0].match(new RegExp(`\\s${name}="([^"]*)"`))?.[1];
  const url = attribute("url");
  const signature = attribute("sparkle:edSignature");
  const length = Number(attribute("length"));
  if (!url || !signature || !Number.isSafeInteger(length) || length <= 0) {
    throw new Error(`${MACOS_APPCAST} enclosure needs url, sparkle:edSignature and length`);
  }
  return { url, signature, length };
}

/** Sparkle signs the archive bytes with Ed25519; both values are base64 as Sparkle prints them. */
export function sparkleSignatureValid(archive: Uint8Array, signature: string, publicKey: string): boolean {
  const key = Buffer.from(publicKey.trim(), "base64");
  const bytes = Buffer.from(signature, "base64");
  if (key.length !== 32) throw new Error("SPARKLE_PUBLIC_KEY must be a base64 Ed25519 public key (32 bytes)");
  if (bytes.length !== 64) return false;
  const jwk = { kty: "OKP", crv: "Ed25519", x: key.toString("base64url") };
  return verifySignature(null, archive, createPublicKey({ key: jwk, format: "jwk" }), bytes);
}

/**
 * Verifies the macOS files before publication. Names prove nothing, so the update ZIP
 * must carry a valid Sparkle signature from SPARKLE_PUBLIC_KEY. The macOS release job
 * creates that signature only after the app and DMG were notarized and stapled.
 */
export function verifyMacReleaseFiles(
  directory: string,
  version: string,
  read: (path: string) => Uint8Array,
  publicKey: string,
  repository: string,
): void {
  for (const name of [macZipName(version), macDmgName(version)]) {
    assertChecksum(read(join(directory, name)), new TextDecoder().decode(read(join(directory, `${name}.sha256`))), name);
  }
  const enclosure = parseAppcastEnclosure(new TextDecoder().decode(read(join(directory, MACOS_APPCAST))), version);
  const expectedUrl = expectedMacZipUrl(repository, version);
  if (enclosure.url !== expectedUrl) {
    throw new Error(`${MACOS_APPCAST} points to ${enclosure.url}, expected ${expectedUrl}`);
  }
  const zip = read(join(directory, macZipName(version)));
  if (enclosure.length !== zip.length) {
    throw new Error(`${MACOS_APPCAST} gives length ${enclosure.length}, but ${macZipName(version)} has ${zip.length} bytes`);
  }
  if (!sparkleSignatureValid(zip, enclosure.signature, publicKey)) {
    throw new Error(`Sparkle signature of ${macZipName(version)} does not match SPARKLE_PUBLIC_KEY; Mac installs would refuse this update`);
  }
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

export function publishedReleaseNotes(version: string, includesMac = false): string {
  const tag = tagForVersion(version);
  const mac = includesMac
    ? [
      "For macOS 14 or later, on Apple Silicon and Intel:",
      "",
      `- \`${macDmgName(version)}\`: disk image; drag Lightcal to Applications.`,
      `- \`${macZipName(version)}\`: the app on its own. Lightcal's macOS updates install this file.`,
      `- \`${MACOS_APPCAST}\`: the macOS update feed.`,
      "",
      "The Mac app is signed with a Developer ID and notarized by Apple.",
      "",
    ]
    : [];
  return [
    `Lightcal ${tag}.`,
    "",
    "Downloads:",
    "",
    `- \`lightcal-${version}-windows-x64-setup.exe\`: installer (recommended).`,
    `- \`lightcal-${tag}-windows-x64.zip\`: portable version, no installation needed.`,
    "- `lightcal.exe`: the app on its own. Lightcal's automatic updates install this file.",
    "",
    "The installer and `lightcal.exe` are digitally signed, and the ZIP contains that same signed `lightcal.exe`. Each download has a `.sha256` checksum next to it.",
    "",
    ...mac,
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
 * LIGHTCAL_UPDATE_CERT_SHA256 in verbatim, so the pin is an ASCII run of hex digits, spaces
 * and colons in any case; each run is normalized the way the app normalizes a pin. Commas
 * end a run, so each entry of a pin list is checked on its own.
 */
export function embedsCertificatePin(executable: Uint8Array, certSha256: string): boolean {
  const cert = normalizeFingerprint(certSha256);
  if (!/^[0-9a-f]{64}$/.test(cert)) throw new Error(`Not a SHA-256 certificate fingerprint: ${certSha256}`);
  const text = Buffer.from(executable).toString("latin1");
  for (const [run] of text.matchAll(/[0-9A-Fa-f][0-9A-Fa-f :]{62,}[0-9A-Fa-f]/g)) {
    if (normalizeFingerprint(run).includes(cert)) return true;
  }
  return false;
}

/**
 * Signing steps of `just sign-draft`, in order. The update pin is checked against the signing
 * certificate before SimplySign signs anything, and against the actual signer before packaging.
 */
export const SIGN_STEPS = [
  "check-pin-before-signing",
  "sign",
  "verify",
  "check-pin-after-signing",
  "package",
  "sign-installer",
] as const;
export type SignStep = (typeof SIGN_STEPS)[number];

/** Steps of `just publish-release`, in order: every check passes before anything changes on GitHub. */
export const PUBLISH_STEPS = [
  "verify-files",
  "verify-mac-files",
  "check-sparkle-key",
  "check-app-version",
  "check-pin",
  "check-setup-version",
  "remove-unsigned",
  "publish",
] as const;
export type PublishStep = (typeof PUBLISH_STEPS)[number];

/** Runs steps in the given order; the record type makes every step need an implementation. */
export function runSteps<S extends string>(order: readonly S[], steps: Record<S, () => void>): void {
  for (const step of order) steps[step]();
}
