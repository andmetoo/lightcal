import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import {
  UNSIGNED_EXE,
  assertChecksum,
  assertFilesExist,
  assertPublishCheckout,
  assertSignDraftCheckout,
  assertUnsignedDraft,
  assertWindowsExecutable,
  deleteAssetArgs,
  editReleaseArgs,
  missingSignedAssets,
  parseChecksum,
  parseReleaseView,
  publishedReleaseNotes,
  publishedReleaseTitle,
  requireStableVersion,
  signedAssetNames,
  signedUploadPaths,
  unsignedAssetNames,
  unsignedAssetsPresent,
  uploadArgs,
  versionFromCheckout,
} from "./release-draft.ts";

const root = join(import.meta.dir, "..");

describe("draft signing names", () => {
  test("keeps the updater asset name for the signed executable only", () => {
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
    expect(() => versionFromCheckout(undefined, null)).toThrow(/git switch --detach/);
  });
});

describe("draft guards", () => {
  test("signs only a clean Windows checkout of the tag", () => {
    expect(() =>
      assertSignDraftCheckout({
        platform: "linux",
        head: "abc",
        tagCommit: "abc",
        dirty: "",
        tag: "v0.2.0",
      }),
    ).toThrow(/Windows PC/);
    expect(() =>
      assertSignDraftCheckout({
        platform: "win32",
        head: "aaaaaaaa",
        tagCommit: "bbbbbbbb",
        dirty: "",
        tag: "v0.2.0",
      }),
    ).toThrow(/git switch --detach v0.2.0/);
    expect(() =>
      assertSignDraftCheckout({
        platform: "win32",
        head: "abc",
        tagCommit: "abc",
        dirty: " M packaging/nsis/lightcal.nsi",
        tag: "v0.2.0",
      }),
    ).toThrow(/Uncommitted changes/);
    expect(() =>
      assertSignDraftCheckout({
        platform: "win32",
        head: "abc",
        tagCommit: "abc",
        dirty: "",
        tag: "v0.2.0",
      }),
    ).not.toThrow();
  });

  test("refuses to publish a missing or already public release", () => {
    expect(() => assertUnsignedDraft(null, "v0.2.0")).toThrow(/unsigned draft/);
    expect(() => assertUnsignedDraft({ isDraft: false, assets: [] }, "v0.2.0")).toThrow(/already published/);
    expect(() => assertPublishCheckout({ head: "aaa", tagCommit: "bbb", tag: "v0.2.0" })).toThrow(
      /git switch --detach v0.2.0/,
    );
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

test("the tag workflow opens a draft and does not publish a prerelease", () => {
  const workflow = readFileSync(join(root, ".github", "workflows", "windows-build.yml"), "utf8");
  const releaseJob = workflow.slice(workflow.indexOf("name: Open unsigned draft release"));
  expect(releaseJob.indexOf("actions/checkout@")).toBeGreaterThan(-1);
  expect(releaseJob.indexOf("actions/checkout@")).toBeLessThan(releaseJob.indexOf("gh release create"));
  expect(workflow).toContain("gh release create");
  expect(workflow).toContain("--draft");
  expect(workflow).toContain("lightcal-unsigned.exe");
  expect(workflow).not.toContain("prerelease: true");
  expect(workflow).not.toContain("softprops/action-gh-release");
});
