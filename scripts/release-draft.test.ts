import { describe, expect, test } from "bun:test";
import { generateKeyPairSync, sign } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  ALLOW_WINDOWS_ONLY,
  assertSparkleKeyContinuity,
  expectedMacZipUrl,
  macShipped,
  macZipVersion,
  parsePublishOptions,
  sparkleKeyFromInfoPlist,
  MACOS_APPCAST,
  type ReleaseView,
  UNSIGNED_EXE,
  assertChecksum,
  assertFilesExist,
  assertReleaseExists,
  assertWindowsExecutable,
  deleteAssetArgs,
  PUBLISH_STEPS,
  SIGN_STEPS,
  editReleaseArgs,
  generatedChangelog,
  embedsCertificatePin,
  hasMacAssets,
  macAssetNames,
  macDmgName,
  macZipName,
  missingMacAssets,
  missingSignedAssets,
  matchingBuildRuns,
  parseAppcastEnclosure,
  parseChecksum,
  parseReleaseView,
  publishedReleaseNotes,
  publishedReleaseTitle,
  requireStableVersion,
  runSteps,
  sha256Hex,
  signedAssetNames,
  signedUploadPaths,
  signingAsset,
  sparkleSignatureValid,
  thumbprintMatches,
  unsignedAssetNames,
  unsignedAssetsPresent,
  uploadArgs,
  versionFromCheckout,
  verifyMacReleaseFiles,
  verifyReleaseFiles,
} from "./release-draft.ts";

const root = join(import.meta.dir, "..");

describe("draft signing names", () => {
  test("keeps compatibility with older unsigned asset names", () => {
    const unsigned = unsignedAssetNames("v0.2.0");
    const signed = signedAssetNames("0.2.0");
    expect(unsigned).not.toContain("lightcal.exe");
    expect(unsigned).toContain(UNSIGNED_EXE);
    expect(signed).toContain("lightcal.exe");
    expect(signed).toContain("lightcal.exe.sha256");
    expect(signed).toContain("lightcal-v0.2.0-windows-x64.zip");
    expect(signed).toContain("lightcal-0.2.0-windows-x64-setup.exe");
    expect(unsigned.filter((name) => signed.includes(name))).toEqual([]);
  });

  test("rejects an unstable version", () => {
    expect(requireStableVersion("v0.2.0")).toBe("0.2.0");
    expect(() => requireStableVersion("v0.2.0-rc.1")).toThrow(/stable version/);
    expect(versionFromCheckout("v0.2.3")).toBe("0.2.3");
    expect(() => versionFromCheckout(undefined)).toThrow(/just sign-draft vX\.Y\.Z/);
  });
});

describe("release inputs", () => {
  test("accepts drafts and public releases regardless of their title", () => {
    expect(() => assertReleaseExists(null, "v0.2.0")).toThrow(/GitHub Actions/);
    for (const isDraft of [false, true]) {
      expect(() => assertReleaseExists({ isDraft, assets: [] }, "v0.2.0")).not.toThrow();
      expect(signingAsset({ isDraft, assets: [{ name: "lightcal.exe" }, { name: "lightcal.exe.sha256" }] })).toBe("lightcal.exe");
    }
  });

  test("accepts the old executable name and requires its matching checksum", () => {
    const assets = [{ name: UNSIGNED_EXE }, { name: `${UNSIGNED_EXE}.sha256` }];
    expect(signingAsset({ isDraft: false, assets })).toBe(UNSIGNED_EXE);
    expect(signingAsset({ isDraft: false, assets: [...assets, { name: "lightcal.exe" }, { name: "lightcal.exe.sha256" }] })).toBe("lightcal.exe");
    expect(() => signingAsset({ isDraft: false, assets: [{ name: "lightcal.exe" }] })).toThrow(/sha256/);
  });

  test("recovers the CI build when release upload failed, but never from another commit or PR", () => {
    const failedRelease = { databaseId: 1, headSha: "tag-commit", status: "completed", event: "push" };
    const runs = [
      { ...failedRelease, databaseId: 2, headSha: "old-tag-commit" },
      { ...failedRelease, databaseId: 3, status: "in_progress" },
      { ...failedRelease, databaseId: 4, event: "pull_request" },
      failedRelease,
    ];
    expect(matchingBuildRuns(runs, "tag-commit")).toEqual([failedRelease]);
  });

  test("publishes only after every signed asset is attached", () => {
    const view = parseReleaseView(
      JSON.stringify({
        isDraft: true,
        url: "https://github.com/andmetoo/lightcal/releases/tag/v0.2.0",
        assets: [
          { name: "lightcal.exe" },
          { name: "lightcal-unsigned.exe" },
          { name: "lightcal-v0.2.0-windows-x64-unsigned.zip" },
        ],
      }),
    );
    expect(missingSignedAssets(view, "0.2.0")).toContain("lightcal.exe.sha256");
    expect(unsignedAssetsPresent(view, "v0.2.0")).toEqual([
      "lightcal-unsigned.exe",
      "lightcal-v0.2.0-windows-x64-unsigned.zip",
    ]);
  });
});

describe("checksum and upload", () => {
  test("checks the downloaded draft executable before signing", () => {
    const bytes = new TextEncoder().encode("MZ-draft");
    const hash = new Bun.CryptoHasher("sha256").update(bytes).digest("hex");
    expect(parseChecksum(`${hash.toUpperCase()}  lightcal-unsigned.exe\n`).hash).toBe(hash);
    expect(assertChecksum(bytes, `${hash}  lightcal-unsigned.exe\n`, UNSIGNED_EXE)).toBe(hash);
    expect(() => assertChecksum(bytes, `${hash}  lightcal.exe\n`, UNSIGNED_EXE)).toThrow(/expected lightcal-unsigned.exe/);
    expect(() => assertChecksum(Uint8Array.from([1, 2, 3]), `${hash}  lightcal-unsigned.exe`, UNSIGNED_EXE)).toThrow(
      /SHA-256 mismatch/,
    );
    expect(() => assertWindowsExecutable(Uint8Array.from([0x4d, 0x5a, 0]))).not.toThrow();
    expect(() => assertWindowsExecutable(Uint8Array.from([1, 2]))).toThrow(/not a Windows executable/);
  });

  test("uploads signed files and publishes with an explicit draft clear", () => {
    const files = signedUploadPaths("/src", "0.2.0");
    expect(files.some((path) => path.endsWith(`${join("target", "dist", "lightcal.exe")}`))).toBe(true);
    expect(uploadArgs("v0.2.0", files)).toEqual(["release", "upload", "v0.2.0", ...files, "--clobber"]);
    expect(deleteAssetArgs("v0.2.0", UNSIGNED_EXE)).toEqual([
      "release",
      "delete-asset",
      "v0.2.0",
      UNSIGNED_EXE,
      "--yes",
    ]);
    expect(editReleaseArgs("v0.2.0", publishedReleaseTitle("0.2.0"), "notes.md")).toContain("--draft=false");
    expect(editReleaseArgs("v0.2.0", publishedReleaseTitle("0.2.0"), "notes.md")).toContain("--prerelease=false");
    expect(publishedReleaseNotes("0.2.0")).toContain("lightcal.exe");
    expect(() => assertFilesExist(["missing.exe"], () => false)).toThrow(/Missing signed file/);
  });
});

test("CI attaches to existing releases without a checkout, visibility changes, or overwriting signed assets", () => {
  const workflow = readFileSync(join(root, ".github", "workflows", "windows-build.yml"), "utf8");
  const releaseJob = workflow.slice(workflow.indexOf("\n  release:"));
  expect(releaseJob).not.toContain("actions/checkout@");
  expect(releaseJob).not.toContain("--clobber");
  expect(releaseJob).not.toContain("gh release edit");
  expect(releaseJob).not.toContain("--json isDraft");
  expect(releaseJob).toContain("GH_REPO:");
  expect(releaseJob).toContain("Keeping existing asset");
  expect(workflow).toContain("gh release create");
  expect(workflow).toContain("--draft");
  expect(releaseJob).toContain('"named/lightcal.exe"');
  expect(workflow).toContain("release_tag:");
  expect(workflow).not.toContain("prerelease: true");
  expect(workflow).not.toContain("softprops/action-gh-release");
});

describe("publication verifies contents rather than filenames", () => {
  const names = ["lightcal.exe", "lightcal-v0.3.0-windows-x64.zip", "lightcal-0.3.0-windows-x64-setup.exe"];
  function fixture() {
    const files = new Map<string, Uint8Array>();
    for (const name of names) {
      const bytes = new TextEncoder().encode(`unsigned ${name}`);
      const hash = new Bun.CryptoHasher("sha256").update(bytes).digest("hex");
      files.set(join("download", name), bytes);
      files.set(join("download", `${name}.sha256`), new TextEncoder().encode(`${hash}  ${name}\n`));
    }
    return files;
  }

  test("rejects ordinary CI files even when all six canonical assets exist", () => {
    const files = fixture();
    expect(() => verifyReleaseFiles("download", "0.3.0", (path) => files.get(path)!,
      () => { throw new Error("Authenticode: NotSigned"); },
      () => { throw new Error("must not reach archive verification"); },
    )).toThrow(/NotSigned/);
  });

  test("validates all checksums, both executable signatures, and the embedded portable payload", () => {
    const files = fixture();
    const verified: string[] = [];
    verifyReleaseFiles("download", "0.3.0", (path) => files.get(path)!, (path) => { verified.push(path); },
      (archive, exe) => { verified.push(archive, exe); });
    expect(verified).toEqual([join("download", names[0]), join("download", names[2]), join("download", names[1]), join("download", names[0])]);
    files.set(join("download", names[2]), new Uint8Array([1, 2, 3]));
    expect(() => verifyReleaseFiles("download", "0.3.0", (path) => files.get(path)!, () => {}, () => {})).toThrow(/SHA-256 mismatch/);
  });
});

test.skipIf(!Bun.which("pwsh"))("portable validation rejects a different application and duplicate payloads without extraction", () => {
  const dir = mkdtempSync(join(tmpdir(), "lightcal-portable-test-"));
  const exe = join(dir, "lightcal.exe");
  const zip = join(dir, "portable.zip");
  const escapedZip = zip.replaceAll("'", "''");
  const escapedExe = exe.replaceAll("'", "''");
  const verify = () => Bun.spawnSync(["pwsh", "-NoLogo", "-NoProfile", "-File", join(root, "scripts", "verify-portable.ps1"), "-Archive", zip, "-Executable", exe]);
  try {
    writeFileSync(exe, "fixture application");
    const created = Bun.spawnSync(["pwsh", "-NoLogo", "-NoProfile", "-Command",
      `Add-Type -AssemblyName System.IO.Compression.FileSystem; $zip = [IO.Compression.ZipFile]::Open('${escapedZip}', 'Create'); try { [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, '${escapedExe}', 'lightcal.exe') | Out-Null } finally { $zip.Dispose() }`,
    ]);
    expect(created.exitCode).toBe(0);
    expect(verify().exitCode).toBe(0);
    writeFileSync(exe, "different application");
    expect(verify().exitCode).not.toBe(0);
    writeFileSync(exe, "fixture application");
    const duplicated = Bun.spawnSync(["pwsh", "-NoLogo", "-NoProfile", "-Command",
      `$zip = [IO.Compression.ZipFile]::Open('${escapedZip}', 'Update'); try { $zip.CreateEntry('nested/lightcal.exe') | Out-Null } finally { $zip.Dispose() }`,
    ]);
    expect(duplicated.exitCode).toBe(0);
    expect(verify().exitCode).not.toBe(0);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}, 60_000); // Cold PowerShell startup on hosted Linux runners can exceed Bun's 5s default.

describe("signer thumbprint pin", () => {
  test("matches case-insensitively and ignores spaces", () => {
    expect(thumbprintMatches("ab cd ef", "ABCDEF")).toBe(true);
    expect(thumbprintMatches("ABCDEF", "abcdef\r\n")).toBe(true);
  });
  test("rejects other signers and an unset pin", () => {
    expect(thumbprintMatches("ABCDEF", "ABCDE0")).toBe(false);
    expect(thumbprintMatches(undefined, "ABCDEF")).toBe(false);
    expect(thumbprintMatches("  ", "")).toBe(false);
  });
});

describe("embedded update pin", () => {
  const cert = "538eff7282ec191b33b46e26daa09ae756f022bae467d2736aca01f69dd9cad6";
  const binary = (embedded: string) => new TextEncoder().encode(`MZ\0\0${embedded}Running application did not exit`);
  test("finds the signer fingerprint in any accepted spelling", () => {
    expect(embedsCertificatePin(binary(cert), cert.toUpperCase())).toBe(true);
    expect(embedsCertificatePin(binary(cert.toUpperCase()), cert)).toBe(true);
    expect(embedsCertificatePin(binary(cert.match(/../g)!.join(":")), cert)).toBe(true);
    expect(embedsCertificatePin(binary(`${"a".repeat(64)},${cert}`), cert)).toBe(true);
    // The app also strips spaces and accepts mixed case.
    expect(embedsCertificatePin(binary(cert.match(/../g)!.join(" ")), cert)).toBe(true);
    expect(embedsCertificatePin(binary(`538EFF${cert.slice(6)}`), cert)).toBe(true);
  });
  test("rejects a build pinned to something else, such as a file checksum", () => {
    const fileChecksum = "f37b579af600613e4b2c54a89ab4395d1e1ad983ecd573e0ad33b49e3e56e678";
    expect(embedsCertificatePin(binary(fileChecksum), cert)).toBe(false);
    expect(embedsCertificatePin(binary(""), cert)).toBe(false);
  });
  test("refuses a malformed fingerprint", () => {
    expect(() => embedsCertificatePin(binary(cert), "F0BBA075C11AFBD3E2798BF517A8F4C3CE74AF6B")).toThrow();
  });
});

describe("release step order", () => {
  const before = <S extends string>(order: readonly S[], a: S, b: S) => order.indexOf(a) < order.indexOf(b);
  test("sign-draft checks the pin before signing and against the signer before packaging", () => {
    expect(before(SIGN_STEPS, "check-pin-before-signing", "sign")).toBe(true);
    expect(before(SIGN_STEPS, "verify", "check-pin-after-signing")).toBe(true);
    expect(before(SIGN_STEPS, "check-pin-after-signing", "package")).toBe(true);
  });
  test("publish-release finishes every check before changing the release", () => {
    for (const check of ["verify-files", "verify-mac-files", "check-sparkle-key", "check-app-version", "check-pin", "check-setup-version"] as const) {
      expect(before(PUBLISH_STEPS, check, "remove-unsigned")).toBe(true);
    }
    expect(PUBLISH_STEPS.at(-1)).toBe("publish");
  });
  test("runSteps runs every step once, in order", () => {
    const ran: string[] = [];
    runSteps(["b", "a"] as const, { a: () => ran.push("a"), b: () => ran.push("b") });
    expect(ran).toEqual(["b", "a"]);
  });
});

describe("macOS release files", () => {
  const view = (names: string[], isDraft = false): ReleaseView => ({ isDraft, assets: names.map((name) => ({ name })) });
  const windows = signedAssetNames("1.4.0");
  const mac = macAssetNames("1.4.0");

  test("Windows-only releases stay publishable until a public release serves the macOS appcast", () => {
    expect(missingMacAssets(view(windows, true), null, "1.4.0")).toEqual([]);
    expect(missingMacAssets(view(windows, true), view(signedAssetNames("1.3.1")), "1.4.0")).toEqual([]);
    // A draft carrying the appcast has not been served to anyone yet.
    expect(missingMacAssets(view(windows, true), view([MACOS_APPCAST], true), "1.4.0")).toEqual([]);
  });

  test("after a signed Mac release, every later release must carry all macOS files", () => {
    const latest = view([...signedAssetNames("1.3.2"), ...macAssetNames("1.3.2")]);
    expect(missingMacAssets(view(windows, true), latest, "1.4.0")).toEqual(mac);
    expect(missingMacAssets(view([...windows, ...mac], true), latest, "1.4.0")).toEqual([]);
  });

  test("partial macOS files are refused even before the first Mac release", () => {
    expect(missingMacAssets(view([...windows, macDmgName("1.4.0")], true), null, "1.4.0")).toEqual(
      mac.filter((name) => name !== macDmgName("1.4.0")),
    );
    expect(hasMacAssets(view([...windows, macDmgName("1.4.0")]), "1.4.0")).toBe(false);
    expect(hasMacAssets(view([...windows, ...mac]), "1.4.0")).toBe(true);
  });

  test("published notes describe the Mac downloads only when they are attached", () => {
    expect(publishedReleaseNotes("1.4.0")).not.toContain("macOS");
    const notes = publishedReleaseNotes("1.4.0", true);
    expect(notes).toContain(macDmgName("1.4.0"));
    expect(notes).toContain(macZipName("1.4.0"));
    expect(notes).toContain("notarized");
  });

  test("Mac files need matching checksums and a valid Sparkle signature from the configured key", () => {
    const directory = mkdtempSync(join(tmpdir(), "lightcal-mac-release-"));
    const { publicKey, privateKey } = generateKeyPairSync("ed25519");
    const sparklePublic = Buffer.from(publicKey.export({ format: "jwk" }).x!, "base64url").toString("base64");
    const zip = new TextEncoder().encode("signed update archive");
    const url = `https://github.com/andmetoo/lightcal/releases/download/v1.4.0/${macZipName("1.4.0")}`;
    const appcast = (fields: { version?: string; url?: string; signature?: string; length?: number }) =>
      `<rss><channel><item><sparkle:version>${fields.version ?? "1.4.0"}</sparkle:version><enclosure url="${fields.url ?? url}" ` +
      `sparkle:edSignature="${fields.signature ?? sign(null, zip, privateKey).toString("base64")}" length="${fields.length ?? zip.length}" type="application/octet-stream"/></item></channel></rss>`;
    const write = (name: string, bytes: Uint8Array | string) => {
      writeFileSync(join(directory, name), bytes);
      if (name !== MACOS_APPCAST) {
        const data = typeof bytes === "string" ? new TextEncoder().encode(bytes) : bytes;
        writeFileSync(join(directory, `${name}.sha256`), `${sha256Hex(data)}  ${name}\n`);
      }
    };
    const check = () => verifyMacReleaseFiles(directory, "1.4.0", readFileSync, sparklePublic, "andmetoo/lightcal");
    try {
      write(macZipName("1.4.0"), zip);
      write(macDmgName("1.4.0"), "disk image");
      write(MACOS_APPCAST, appcast({}));
      expect(check).not.toThrow();

      write(MACOS_APPCAST, appcast({ url: url.replaceAll("1.4.0", "1.3.2") }));
      expect(check).toThrow(/points to/);
      // Right path, wrong repository or host.
      write(MACOS_APPCAST, appcast({ url: url.replace("andmetoo/lightcal", "someone/lightcal-fork") }));
      expect(check).toThrow(/points to/);
      write(MACOS_APPCAST, appcast({ url: url.replace("https://github.com", "https://downloads.example") }));
      expect(check).toThrow(/points to/);
      write(MACOS_APPCAST, appcast({ version: "1.3.2" }));
      expect(check).toThrow(/does not announce version 1.4.0/);
      write(MACOS_APPCAST, appcast({ length: zip.length + 1 }));
      expect(check).toThrow(/gives length/);
      write(MACOS_APPCAST, appcast({ signature: Buffer.alloc(64).toString("base64") }));
      expect(check).toThrow(/Sparkle signature/);

      const other = generateKeyPairSync("ed25519").privateKey;
      write(MACOS_APPCAST, appcast({ signature: sign(null, zip, other).toString("base64") }));
      expect(check).toThrow(/Sparkle signature/);

      // A replaced ZIP with a recomputed checksum still fails the signature.
      write(macZipName("1.4.0"), "unsigned replacement!");
      write(MACOS_APPCAST, appcast({ length: "unsigned replacement!".length }));
      expect(check).toThrow(/Sparkle signature/);

      write(macZipName("1.4.0"), zip);
      write(MACOS_APPCAST, appcast({}));
      writeFileSync(join(directory, macDmgName("1.4.0")), "tampered");
      expect(check).toThrow(/SHA-256 mismatch/);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  test("appcast parsing rejects missing or repeated enclosures and malformed keys", () => {
    expect(() => parseAppcastEnclosure("<sparkle:version>1.4.0</sparkle:version>", "1.4.0")).toThrow(/exactly one enclosure/);
    const one = `<enclosure url="https://example.test/a.zip" sparkle:edSignature="c2ln" length="5"/>`;
    expect(() => parseAppcastEnclosure(`<sparkle:version>1.4.0</sparkle:version>${one}${one}`, "1.4.0")).toThrow(/exactly one enclosure/);
    expect(parseAppcastEnclosure(`<sparkle:version>1.4.0</sparkle:version>${one}`, "1.4.0")).toEqual({
      url: "https://example.test/a.zip", signature: "c2ln", length: 5,
    });
    expect(() => sparkleSignatureValid(new Uint8Array(1), "c2ln", "c2hvcnQ=")).toThrow(/32 bytes/);
  });

  test("an explicit override allows a Windows-only release, but never partial macOS files", () => {
    const latest = view([...signedAssetNames("1.3.2"), ...macAssetNames("1.3.2")]);
    expect(missingMacAssets(view(windows, true), latest, "1.4.0", true)).toEqual([]);
    expect(missingMacAssets(view([...windows, MACOS_APPCAST], true), latest, "1.4.0", true)).toEqual(
      mac.filter((name) => name !== MACOS_APPCAST),
    );
    expect(macShipped(latest)).toBe(true);
    expect(macShipped(view([MACOS_APPCAST], true))).toBe(false);
    expect(parsePublishOptions([])).toEqual({ allowWindowsOnly: false });
    expect(parsePublishOptions([ALLOW_WINDOWS_ONLY])).toEqual({ allowWindowsOnly: true });
    expect(() => parsePublishOptions(["--allow-windows"])).toThrow(/Unknown publish-release option/);
  });

  test("the Sparkle key must match the one installed Macs trust and the one this release embeds", () => {
    const plist = (key: string) => `<plist><dict><key>SUFeedURL</key><string>x</string><key>SUPublicEDKey</key>\n<string>${key}</string></dict></plist>`;
    expect(sparkleKeyFromInfoPlist(plist("AAAA"))).toBe("AAAA");
    expect(() => sparkleKeyFromInfoPlist(plist(""))).toThrow(/no SUPublicEDKey/);
    expect(() => sparkleKeyFromInfoPlist("<plist><dict></dict></plist>")).toThrow(/no SUPublicEDKey/);

    expect(() => assertSparkleKeyContinuity("KEY", "KEY", null)).not.toThrow();
    expect(() => assertSparkleKeyContinuity("KEY", "KEY", "KEY")).not.toThrow();
    expect(() => assertSparkleKeyContinuity("NEW", "NEW", "OLD")).toThrow(/latest public Mac release/);
    expect(() => assertSparkleKeyContinuity("KEY", "OTHER", "KEY")).toThrow(/own next update/);

    expect(macZipVersion(view([...signedAssetNames("1.3.2"), ...macAssetNames("1.3.2")]))).toBe("1.3.2");
    expect(macZipVersion(view(signedAssetNames("1.3.2")))).toBeNull();
    expect(macZipVersion(null)).toBeNull();
    expect(expectedMacZipUrl("andmetoo/lightcal", "1.4.0")).toBe(
      "https://github.com/andmetoo/lightcal/releases/download/v1.4.0/lightcal-1.4.0-macos-universal.zip",
    );
  });

  test("publish-release checks the Mac files before changing the release", () => {
    expect(PUBLISH_STEPS.indexOf("verify-mac-files")).toBeLessThan(PUBLISH_STEPS.indexOf("remove-unsigned"));
    expect(PUBLISH_STEPS.indexOf("check-sparkle-key")).toBeLessThan(PUBLISH_STEPS.indexOf("remove-unsigned"));
  });
});

describe("published notes keep GitHub's generated changelog", () => {
  const generated = [
    "## What's Changed",
    "* Add macOS support by @someone in https://github.com/andmetoo/lightcal/pull/73",
    "",
    "**Full Changelog**: https://github.com/andmetoo/lightcal/compare/v1.3.1...v1.3.2",
  ].join("\n");

  test("the changelog follows the download section and is kept once on republish", () => {
    const notes = publishedReleaseNotes("1.3.2", true, generatedChangelog(`Draft text.\r\n\r\n${generated}\r\n`));
    expect(notes.indexOf("For macOS 14")).toBeLessThan(notes.indexOf("## What's Changed"));
    expect(notes).toContain("**Full Changelog**");
    // Publishing an already published release must not duplicate it.
    const again = publishedReleaseNotes("1.3.2", true, generatedChangelog(notes));
    expect(again).toBe(notes);
    expect(again.split("## What's Changed").length).toBe(2);
  });

  test("notes without a generated changelog are unchanged", () => {
    expect(generatedChangelog(undefined)).toBe("");
    expect(generatedChangelog("Signed macOS build prepared for review.")).toBe("");
    expect(generatedChangelog("**Full Changelog**: https://example.test/compare")).toBe("**Full Changelog**: https://example.test/compare");
    expect(publishedReleaseNotes("1.3.2", false, "")).toBe(publishedReleaseNotes("1.3.2"));
  });
});
