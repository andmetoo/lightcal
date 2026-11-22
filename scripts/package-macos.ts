/** Build a universal macOS bundle. Release mode requires configured signing and Sparkle. */
import { mkdir, mkdtemp, cp, rm, symlink, writeFile, readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { DMG_LAYOUT, finderLayoutScript } from "./macos-dmg";
import { validateMacDeploymentTarget } from "./macos-deployment";
import { notarySubmitCommand } from "./macos-notarization";
import { prepareMacBundle } from "./macos-bundle";
import { appSigningCommand, assertCalendarsEntitlement, sparkleSigningCommands } from "./macos-signing";

process.env.MACOSX_DEPLOYMENT_TARGET = "14.0";
const args = process.argv.slice(2);
if (args.includes("--help")) {
  console.log("Usage: just installer [--release]\nBuild a local unsigned universal app and DMG. --release requires signing, notarization and Sparkle configuration.");
  process.exit(0);
}
if (args.some(arg => arg !== "--release" && arg !== "--dmg")) {
  console.error("Unknown macOS packaging option. Use just installer --help; -SkipBuild is Windows-only.");
  process.exit(2);
}
const release = process.argv.includes("--release");
const root = resolve(import.meta.dir, "..");
const out = join(root, "target/macos");
const app = join(out, "Lightcal.app");
const contents = join(app, "Contents");
const workspacePackage = (await readFile(join(root, "Cargo.toml"), "utf8")).match(/\[workspace\.package\][\s\S]*?(?=\n\[|$)/)?.[0];
const cargoVersion = workspacePackage?.match(/^\s*version\s*=\s*"([^"]+)"/m)?.[1];
const version = (process.env.LIGHTCAL_RELEASE_VERSION || cargoVersion || "1.3.3").replace(/^v/, "");
if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error("Expected stable x.y.z version");
const run = async (...args: string[]) => {
  // Bun's default inherited environment can omit mutations made above.
  // Pass the current environment so rustc receives the deployment target.
  const child = Bun.spawn(args, { cwd: root, env: process.env, stdout: "inherit", stderr: "inherit" });
  if (await child.exited !== 0) throw new Error(`Failed: ${args[0]}`);
};
const required = (name: string) => {
  const value = process.env[name];
  if (!value) throw new Error(`Missing release configuration: ${name}`);
  return value;
};
if (release) for (const key of ["APPLE_SIGNING_IDENTITY", "APPLE_NOTARY_PROFILE", "APPLE_SIGNING_KEYCHAIN", "SPARKLE_DIR", "SPARKLE_PUBLIC_KEY", "SPARKLE_PRIVATE_KEY_FILE", "GITHUB_REPOSITORY", "LIGHTCAL_MICROSOFT_CLIENT_ID", "LIGHTCAL_GOOGLE_CLIENT_ID", "LIGHTCAL_GOOGLE_CLIENT_SECRET"]) required(key);
for (const arch of ["aarch64", "x86_64"]) await run("cargo", "build", "--release", "--locked", "-p", "lightcal", "--target", `${arch}-apple-darwin`);
await prepareMacBundle(app);
await run("lipo", "-create", ...["aarch64", "x86_64"].map(arch => `target/${arch}-apple-darwin/release/lightcal`), "-output", join(contents, "MacOS/lightcal"));
const inspection = Bun.spawn(["xcrun", "vtool", "-show-build", join(contents, "MacOS/lightcal")], {
  cwd: root, env: process.env, stdout: "pipe", stderr: "inherit",
});
const buildVersions = await new Response(inspection.stdout).text();
if (await inspection.exited !== 0) throw new Error("Could not inspect executable deployment targets.");
validateMacDeploymentTarget(buildVersions);
for (const name of ["LICENSE", "THIRD_PARTY_NOTICES.md"]) await cp(join(root, name), join(contents, "Resources", name));
const iconSource = join(out, "app-icon.png");
await run("target/" + (process.arch === "arm64" ? "aarch64" : "x86_64") + "-apple-darwin/release/lightcal", "--export-app-icon-png", iconSource);
const icons = join(out, "Lightcal.iconset");
await mkdir(icons, { recursive: true });
for (const size of [16, 32, 128, 256, 512]) for (const scale of [1, 2]) {
  await run("sips", "-s", "format", "png", "-z", String(size * scale), String(size * scale), iconSource, "--out", join(icons, `icon_${size}x${size}${scale === 2 ? "@2x" : ""}.png`));
}
await run("iconutil", "-c", "icns", icons, "-o", join(contents, "Resources/Lightcal.icns"));
const xml = (value: string) => value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll('"', "&quot;");
const repo = process.env.GITHUB_REPOSITORY || "andmetoo/lightcal";
if (!/^[\w.-]+\/[\w.-]+$/.test(repo)) throw new Error("Invalid repository");
await writeFile(join(contents, "Info.plist"), `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>app.koona.lightcal</string>
<key>CFBundleName</key><string>Lightcal</string>
<key>CFBundleExecutable</key><string>lightcal</string>
<key>CFBundleIconFile</key><string>Lightcal.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>${version}</string>
<key>CFBundleShortVersionString</key><string>${version}</string>
<key>LSMinimumSystemVersion</key><string>14.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSCalendarsFullAccessUsageDescription</key><string>Lightcal displays your selected calendars and events. Lightcal never changes your calendars.</string>
<key>SUFeedURL</key><string>https://github.com/${repo}/releases/latest/download/lightcal-macos-appcast.xml</string>
<key>SUPublicEDKey</key><string>${xml(process.env.SPARKLE_PUBLIC_KEY || "")}</string>
<key>SUEnableAutomaticChecks</key><false/>
<key>SUAutomaticallyUpdate</key><false/>
<key>SUEnableSystemProfiling</key><false/>
</dict></plist>`);
/** Runs a command and returns its stdout; stderr stays visible in the build log. */
const output = async (...args: string[]) => {
  const child = Bun.spawn(args, { cwd: root, env: process.env, stdout: "pipe", stderr: "inherit" });
  const text = await new Response(child.stdout).text();
  if (await child.exited !== 0) throw new Error(`Failed: ${args[0]}`);
  return text;
};
const warn = (message: string) => console.warn(process.env.GITHUB_ACTIONS === "true" ? `::warning::${message}` : message);

/**
 * Disk image with the designed Finder window: background art, 128 pt icons placed on it,
 * the Lightcal volume icon and no toolbar. Finder writes the layout into .DS_Store on a
 * writable image, which is then compressed. If Finder cannot be scripted, the image is
 * still built and works, with Finder's default layout.
 */
async function createDiskImage(): Promise<string> {
  const staging = await mkdtemp(join(out, "dmg-content-"));
  const dmg = join(out, `lightcal-${version}-macos-universal.dmg`);
  const writable = join(out, "dmg-writable.dmg");
  const volume = `/Volumes/${DMG_LAYOUT.volumeName}`;
  let device: string | undefined;
  try {
    await cp(app, join(staging, DMG_LAYOUT.app.name), { recursive: true, verbatimSymlinks: true });
    await symlink("/Applications", join(staging, DMG_LAYOUT.applications.name));
    await mkdir(join(staging, dirname(DMG_LAYOUT.background)));
    await run("tiffutil", "-cathidpicheck", join(root, "assets/macos/dmg-background.png"), join(root, "assets/macos/dmg-background@2x.png"),
      "-out", join(staging, DMG_LAYOUT.background));
    if (existsSync(volume)) throw new Error(`Eject ${volume} first; Finder lays out the new image by its volume name.`);
    const megabytes = Number((await output("du", "-sm", staging)).split(/\s/)[0]) + 20;
    await rm(writable, { force: true });
    await run("hdiutil", "create", "-volname", DMG_LAYOUT.volumeName, "-srcfolder", staging, "-fs", "HFS+",
      "-format", "UDRW", "-size", `${megabytes}m`, "-ov", writable);
    const attached = await output("hdiutil", "attach", "-readwrite", "-noverify", "-noautoopen", writable);
    device = attached.match(/^(\/dev\/disk\d+)\s/m)?.[1];
    if (!device || !attached.includes(volume)) throw new Error(`The writable image did not mount at ${volume}.`);
    const finder = Bun.spawn(["osascript"], { stdin: new TextEncoder().encode(finderLayoutScript()), stdout: "ignore", stderr: "pipe" });
    const timer = setTimeout(() => finder.kill(), 120_000);
    const finderError = await new Response(finder.stderr).text();
    clearTimeout(timer);
    if (await finder.exited !== 0) warn(`Finder could not lay out the disk image window; it keeps the default layout. ${finderError.trim()}`);
    // After the layout: Finder drops a volume icon that is already present when it rewrites the view.
    await cp(join(contents, "Resources/Lightcal.icns"), join(volume, ".VolumeIcon.icns"));
    await run("xcrun", "SetFile", "-a", "C", volume);
    await rm(join(volume, ".fseventsd"), { recursive: true, force: true });
    await run("sync");
    for (let attempt = 1; ; attempt++) {
      const detach = Bun.spawn(["hdiutil", "detach", device, ...(attempt === 5 ? ["-force"] : [])], { stdout: "ignore", stderr: "ignore" });
      if (await detach.exited === 0) break;
      if (attempt === 5) throw new Error(`Could not eject ${volume}.`);
      await Bun.sleep(2000);
    }
    device = undefined;
    await run("hdiutil", "convert", writable, "-format", "UDZO", "-imagekey", "zlib-level=9", "-ov", "-o", dmg);
  } finally {
    if (device) Bun.spawnSync(["hdiutil", "detach", device, "-force"]);
    await rm(staging, { recursive: true, force: true });
    await rm(writable, { force: true });
  }
  return dmg;
}
if (release) {
  await mkdir(join(contents, "Frameworks"), { recursive: true });
  await cp(join(required("SPARKLE_DIR"), "Sparkle.framework"), join(contents, "Frameworks/Sparkle.framework"), { recursive: true, verbatimSymlinks: true });
  await cp(join(required("SPARKLE_DIR"), "LICENSE"), join(contents, "Resources/Sparkle-LICENSE"));
  // Sign each nested executable and bundle before the enclosing application.
  const identity = required("APPLE_SIGNING_IDENTITY");
  for (const command of sparkleSigningCommands(join(contents, "Frameworks/Sparkle.framework"), identity)) await run(...command);
  await run(...appSigningCommand(app, identity, join(root, "packaging/macos/Lightcal.entitlements")));
  const entitlements = Bun.spawn(["codesign", "-d", "--entitlements", "-", "--xml", app], { stdout: "pipe", stderr: "pipe" });
  const granted = await new Response(entitlements.stdout).text();
  if (await entitlements.exited !== 0) throw new Error("Could not read the signed app's entitlements.");
  assertCalendarsEntitlement(granted);
  const zip = join(out, `lightcal-${version}-macos-universal.zip`);
  await run("ditto", "-c", "-k", "--keepParent", app, zip);
  await run(...notarySubmitCommand(zip, required("APPLE_NOTARY_PROFILE"), required("APPLE_SIGNING_KEYCHAIN")));
  await run("xcrun", "stapler", "staple", app);
  await run("codesign", "--verify", "--deep", "--strict", app);
  await run("spctl", "--assess", "--type", "execute", app);
  await run("ditto", "-c", "-k", "--keepParent", app, zip);
  const dmg = await createDiskImage();
  await run("codesign", "--timestamp", "--sign", required("APPLE_SIGNING_IDENTITY"), dmg);
  await run(...notarySubmitCommand(dmg, required("APPLE_NOTARY_PROFILE"), required("APPLE_SIGNING_KEYCHAIN")));
  await run("xcrun", "stapler", "staple", dmg);
  const signer = Bun.spawn([join(required("SPARKLE_DIR"), "bin/sign_update"), "--ed-key-file", required("SPARKLE_PRIVATE_KEY_FILE"), zip], { stdout: "pipe", stderr: "inherit" });
  const signature = await new Response(signer.stdout).text();
  if (await signer.exited !== 0 || !/^sparkle:edSignature="[A-Za-z0-9+/=]+" length="\d+"\s*$/.test(signature.trim())) throw new Error("Update signing failed");
  await writeFile(join(out, "lightcal-macos-appcast.xml"), `<?xml version="1.0" encoding="utf-8"?><rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle"><channel><title>Lightcal macOS</title><item><title>Lightcal ${version}</title><sparkle:version>${version}</sparkle:version><sparkle:minimumSystemVersion>14.0</sparkle:minimumSystemVersion><enclosure url="https://github.com/${repo}/releases/download/v${version}/lightcal-${version}-macos-universal.zip" ${signature.trim()} type="application/octet-stream"/></item></channel></rss>`);
  for (const file of [zip, dmg]) await writeFile(`${file}.sha256`, `${new Bun.CryptoHasher("sha256").update(await readFile(file)).digest("hex")}  ${file.split("/").at(-1)}\n`);
} else if (args.includes("--dmg")) {
  const dmg = await createDiskImage();
  await writeFile(`${dmg}.sha256`, `${new Bun.CryptoHasher("sha256").update(await readFile(dmg)).digest("hex")}  ${dmg.split("/").at(-1)}\n`);
  console.log(`Prepared ${dmg} (unsigned local review image)`);
}
console.log(`Prepared ${app}${release ? " with signed release artifacts" : " (unsigned review build; updates disabled)"}`);
