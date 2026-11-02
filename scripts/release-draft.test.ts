import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  UNSIGNED_EXE,
  assertChecksum,
  assertFilesExist,
  assertReleaseExists,
  assertWindowsExecutable,
  deleteAssetArgs,
  PUBLISH_STEPS,
  SIGN_STEPS,
  editReleaseArgs,
  embedsCertificatePin,
  missingSignedAssets,
  matchingBuildRuns,
  parseChecksum,
  parseReleaseView,
  publishedReleaseNotes,
  publishedReleaseTitle,
  requireStableVersion,
  runSteps,
  signedAssetNames,
  signedUploadPaths,
  signingAsset,
  thumbprintMatches,
  unsignedAssetNames,
  unsignedAssetsPresent,
  uploadArgs,
  versionFromCheckout,
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
    for (const check of ["verify-files", "check-app-version", "check-pin", "check-setup-version"] as const) {
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
