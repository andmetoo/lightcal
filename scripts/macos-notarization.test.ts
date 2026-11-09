import { expect, test } from "bun:test";
import { notarySubmitCommand } from "./macos-notarization";

test("ZIP and DMG notarization explicitly use the temporary profile keychain", () => {
  for (const artifact of ["/tmp/release files/Lightcal.zip", "/tmp/release files/Lightcal.dmg"]) {
    expect(notarySubmitCommand(artifact, "lightcal-ci", "/tmp/signing keys/lightcal.keychain-db")).toEqual([
      "xcrun", "notarytool", "submit", artifact, "--keychain-profile", "lightcal-ci",
      "--keychain", "/tmp/signing keys/lightcal.keychain-db", "--wait",
    ]);
  }
});
