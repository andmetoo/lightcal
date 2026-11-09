import { expect, test } from "bun:test";
import { installerCommand } from "./installer";

test("macOS installer uses the universal DMG builder without PowerShell", () => {
  expect(installerCommand("darwin", [])).toEqual(["bun", "scripts/package-macos.ts", "--dmg"]);
});

test("installer routing preserves native arguments without shell evaluation", () => {
  const args = ["-SkipBuild", "-OutputDirectory", "a path/with spaces;$(literal)"];
  expect(installerCommand("win32", args)).toEqual([
    "pwsh", "-NoLogo", "-NoProfile", "-File", "scripts/package-nsis.ps1", ...args,
  ]);
  expect(installerCommand("darwin", ["--release"])).toEqual([
    "bun", "scripts/package-macos.ts", "--dmg", "--release",
  ]);
});

test("unsupported installer hosts fail before trying platform tools", () => {
  expect(() => installerCommand("linux", [])).toThrow("macOS or Windows");
});
