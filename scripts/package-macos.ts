/** Build a universal macOS bundle. Release mode requires configured signing and Sparkle. */
import { mkdir, mkdtemp, cp, rm, symlink, writeFile, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { validateMacDeploymentTarget } from "./macos-deployment";
import { notarySubmitCommand } from "./macos-notarization";
import { prepareMacBundle } from "./macos-bundle";

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
const version = (process.env.LIGHTCAL_RELEASE_VERSION || "1.2.2").replace(/^v/, "");
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
async function createDiskImage(): Promise<string> {
  const staging = await mkdtemp(join(out, "dmg-content-"));
  const dmg = join(out, `lightcal-${version}-macos-universal.dmg`);
  try {
    await cp(app, join(staging, "Lightcal.app"), { recursive: true, verbatimSymlinks: true });
    await symlink("/Applications", join(staging, "Applications"));
    await run("hdiutil", "create", "-volname", "Lightcal", "-srcfolder", staging, "-ov", "-format", "UDZO", dmg);
  } finally {
    await rm(staging, { recursive: true, force: true });
  }
  return dmg;
}
if (release) {
  await mkdir(join(contents, "Frameworks"), { recursive: true });
  await cp(join(required("SPARKLE_DIR"), "Sparkle.framework"), join(contents, "Frameworks/Sparkle.framework"), { recursive: true, verbatimSymlinks: true });
  await cp(join(required("SPARKLE_DIR"), "LICENSE"), join(contents, "Resources/Sparkle-LICENSE"));
  // Sign each nested executable and bundle before the enclosing application.
  const framework = join(contents, "Frameworks/Sparkle.framework");
  for (const path of ["Versions/B/XPCServices/Downloader.xpc", "Versions/B/XPCServices/Installer.xpc", "Versions/B/Autoupdate", "Versions/B/Updater.app", ""]) {
    await run("codesign", "--force", "--timestamp", "--options", "runtime", "--sign", required("APPLE_SIGNING_IDENTITY"), join(framework, path));
  }
  await run("codesign", "--force", "--timestamp", "--options", "runtime", "--sign", required("APPLE_SIGNING_IDENTITY"), app);
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
