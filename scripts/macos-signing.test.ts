import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import {
  appSigningCommand,
  assertCalendarsEntitlement,
  CALENDARS_ENTITLEMENT,
  sparkleSigningCommands,
} from "./macos-signing";

const identity = "Developer ID Application: Example (TEAMID1234)";
const entitlementsFile = join(import.meta.dir, "../packaging/macos/Lightcal.entitlements");

test("the app is signed with the checked-in hardened-runtime entitlements", () => {
  expect(appSigningCommand("/tmp/out/Lightcal.app", identity, entitlementsFile)).toEqual([
    "codesign", "--force", "--timestamp", "--options", "runtime", "--sign", identity,
    "--entitlements", entitlementsFile, "/tmp/out/Lightcal.app",
  ]);
});

test("the entitlements grant calendar access and nothing that weakens the runtime", async () => {
  const plist = await readFile(entitlementsFile, "utf8");
  expect(() => assertCalendarsEntitlement(plist)).not.toThrow();
  for (const key of ["get-task-allow", "disable-library-validation", "allow-jit", "allow-unsigned-executable-memory", "app-sandbox"]) {
    expect(plist).not.toContain(key);
  }
});

test("Sparkle is signed innermost first and Downloader.xpc keeps its entitlements", () => {
  const framework = "/tmp/out/Lightcal.app/Contents/Frameworks/Sparkle.framework";
  const commands = sparkleSigningCommands(framework, identity);
  expect(commands.map(command => command.at(-1))).toEqual([
    `${framework}/Versions/B/XPCServices/Installer.xpc`,
    `${framework}/Versions/B/XPCServices/Downloader.xpc`,
    `${framework}/Versions/B/Autoupdate`,
    `${framework}/Versions/B/Updater.app`,
    framework,
  ]);
  for (const command of commands) {
    expect(command.slice(0, 7)).toEqual(["codesign", "--force", "--timestamp", "--options", "runtime", "--sign", identity]);
    expect(command.includes("--preserve-metadata=entitlements")).toBe(command.at(-1)!.endsWith("Downloader.xpc"));
  }
});

test("a signed app without the calendars entitlement is rejected", () => {
  expect(() => assertCalendarsEntitlement("<plist><dict></dict></plist>")).toThrow(CALENDARS_ENTITLEMENT);
  expect(() => assertCalendarsEntitlement(`<plist><dict><key>${CALENDARS_ENTITLEMENT}</key><false/></dict></plist>`)).toThrow(CALENDARS_ENTITLEMENT);
  expect(() => assertCalendarsEntitlement(`<plist><dict><key>${CALENDARS_ENTITLEMENT.replace("calendars", "calendarsX")}</key><true/></dict></plist>`)).toThrow();
  expect(() => assertCalendarsEntitlement(`<plist><dict><key>${CALENDARS_ENTITLEMENT}</key><true/></dict></plist>`)).not.toThrow();
});

test("the macOS release job replaces its own files only on draft releases", async () => {
  const workflow = await readFile(join(import.meta.dir, "../.github/workflows/macos-build.yml"), "utf8");
  const attach = workflow.slice(workflow.indexOf("- name: Attach to draft release"), workflow.indexOf("- name: Build unsigned universal DMG\n        if: ${{ steps.signing"));
  expect(attach).toContain("refusing asset changes");
  expect(attach.indexOf("refusing asset changes")).toBeLessThan(attach.indexOf("gh release upload"));
  expect(attach).toMatch(/gh release upload "\$RELEASE_TAG" --clobber /);
});
