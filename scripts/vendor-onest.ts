// Prepare the pinned Onest font files with attribution; never runs downloaded package code.
// Usage: bun scripts/vendor-onest.ts [destination]   (default: design/fonts/onest)
import { createHash } from "node:crypto";
import { mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { gunzipSync, inflateRawSync } from "node:zlib";

const ROOT = resolve(import.meta.dir, "..");
const DESTINATION = resolve(process.argv[2] ?? join(ROOT, "design", "fonts", "onest"));
const PACKAGE_URL = "https://registry.npmjs.org/@fontsource/onest/-/onest-5.3.1.tgz";
const PACKAGE_INTEGRITY = "sha512-2uA9HUzQHBB8H7T2R6inmrnqPofB6+svATHM/huIG640duJr4rm+uanmAIheTTlbZvRC0ujMtf2T4U4RX51oEQ==";
const RELEASE_URL = "https://github.com/simpals/onest/releases/download/2.001/onest-2.001.zip";
const RELEASE_SHA256 = "17b59e19c349e603b7d113a596b6d8e08427e97a7b5235668b69e9d8a06a4267";

const hash = (algorithm: string, bytes: Uint8Array, encoding: "hex" | "base64") =>
  createHash(algorithm).update(bytes).digest(encoding);

async function download(url: string): Promise<Uint8Array> {
  const response = await fetch(url, { headers: { "User-Agent": "Lightcal-design-font-vendor" }, signal: AbortSignal.timeout(60_000) });
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

/** Regular files from a ustar archive, by path. Only explicitly named members are used. */
function untar(bytes: Uint8Array): Map<string, Uint8Array> {
  const text = (start: number, length: number) =>
    new TextDecoder().decode(bytes.subarray(start, start + length)).replace(/\0.*$/s, "");
  const files = new Map<string, Uint8Array>();
  for (let offset = 0; offset + 512 <= bytes.length && bytes[offset] !== 0;) {
    const name = text(offset, 100);
    const prefix = text(offset + 345, 155);
    const size = parseInt(text(offset + 124, 12).trim() || "0", 8);
    const type = bytes[offset + 156];
    offset += 512;
    if (type === 0x30 || type === 0) files.set(prefix ? `${prefix}/${name}` : name, bytes.slice(offset, offset + size));
    offset += Math.ceil(size / 512) * 512;
  }
  return files;
}

/** Reads one member of a ZIP archive (stored or deflated) by its exact path. */
function zipMember(bytes: Uint8Array, wanted: string): Uint8Array {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let end = bytes.length - 22;
  while (end >= 0 && view.getUint32(end, true) !== 0x06054b50) end--;
  if (end < 0) throw new Error("ZIP end record not found");
  let entry = view.getUint32(end + 16, true);
  for (let count = view.getUint16(end + 10, true); count > 0; count--) {
    const method = view.getUint16(entry + 10, true);
    const compressed = view.getUint32(entry + 20, true);
    const nameLength = view.getUint16(entry + 28, true);
    const name = new TextDecoder().decode(bytes.subarray(entry + 46, entry + 46 + nameLength));
    if (name === wanted) {
      const local = view.getUint32(entry + 42, true);
      const start = local + 30 + view.getUint16(local + 26, true) + view.getUint16(local + 28, true);
      const data = bytes.subarray(start, start + compressed);
      if (method === 0) return data.slice();
      if (method === 8) return new Uint8Array(inflateRawSync(data));
      throw new Error(`${name}: unsupported ZIP compression ${method}`);
    }
    entry += 46 + nameLength + view.getUint16(entry + 30, true) + view.getUint16(entry + 32, true);
  }
  throw new Error(`${wanted} is missing from the ZIP archive`);
}

const packageBytes = await download(PACKAGE_URL);
if (`sha512-${hash("sha512", packageBytes, "base64")}` !== PACKAGE_INTEGRITY) throw new Error("Fontsource package integrity mismatch");
const tar = untar(gunzipSync(packageBytes));
const member = (path: string) => tar.get(path) ?? (() => { throw new Error(`${path} is missing from the package`); })();
const metadata = JSON.parse(new TextDecoder().decode(member("package/metadata.json")));
if (metadata.family !== "Onest") throw new Error("Unexpected font family");
// Subset files stay in ignored scratch storage for glyph validation.
const scratch = join(ROOT, "target", "onest-proof");
mkdirSync(scratch, { recursive: true });
for (const weight of [400, 500, 600, 700]) {
  for (const subset of ["latin", "cyrillic"]) {
    const name = `onest-${subset}-${weight}-normal.woff`;
    writeFileSync(join(scratch, name), member(`package/files/${name}`));
  }
}

const release = await download(RELEASE_URL);
if (hash("sha256", release, "hex") !== RELEASE_SHA256) throw new Error("Upstream release integrity mismatch");
mkdirSync(DESTINATION, { recursive: true });
const selected: Record<string, string> = { "OFL.txt": "OFL.txt" };
for (const style of ["Regular", "Medium", "SemiBold", "Bold"]) selected[`fonts/ttf/Onest-${style}.ttf`] = `Onest-${style}.ttf`;
const filesSha256: Record<string, string> = {};
for (const [source, file] of Object.entries(selected)) {
  const data = zipMember(release, `onest-2.001/${source}`);
  writeFileSync(join(DESTINATION, file), data);
  filesSha256[file] = hash("sha256", data, "hex");
}
const manifest = {
  family: "Onest",
  selection: "Fontsource 5.3.x, pinned to 5.3.1",
  fontsource: { package: "@fontsource/onest", version: "5.3.1", url: PACKAGE_URL, integrity: PACKAGE_INTEGRITY, metadata },
  nativeSource: { version: "2.001", url: RELEASE_URL, sha256: RELEASE_SHA256, sourceCommit: "8739b1910618a15335e4cc48842052d0ee739ade" },
  filesSha256,
};
writeFileSync(join(DESTINATION, "source.json"), `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Prepared ${Object.keys(filesSha256).length} verified font/license files and source.json in ${DESTINATION}`);
