import { expect, test } from "bun:test";
import { notaryApiKey, notaryStoreCredentialsCommand, notarySubmitCommand } from "./macos-notarization";

test("ZIP and DMG notarization explicitly use the temporary profile keychain", () => {
  for (const artifact of ["/tmp/release files/Lightcal.zip", "/tmp/release files/Lightcal.dmg"]) {
    expect(notarySubmitCommand(artifact, "lightcal-ci", "/tmp/signing keys/lightcal.keychain-db")).toEqual([
      "xcrun", "notarytool", "submit", artifact, "--keychain-profile", "lightcal-ci",
      "--keychain", "/tmp/signing keys/lightcal.keychain-db", "--wait",
    ]);
  }
});

const pem = "-----BEGIN PRIVATE KEY-----\nMIGTAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBHkwdwIBAQQg\n-----END PRIVATE KEY-----";
const apiEnv = (overrides: Record<string, string | undefined> = {}) => ({
  APPLE_API_KEY: "ABCDE12345",
  APPLE_API_ISSUER: "69a6de7e-1234-47e3-e053-5b8c7c11a4d1",
  APPLE_API_KEY_P8: pem,
  ...overrides,
});

test("notarization stores an App Store Connect API key profile, not an Apple ID", () => {
  const key = notaryApiKey(apiEnv());
  expect(notaryStoreCredentialsCommand("lightcal-ci", "/tmp/run/key.p8", key, "/tmp/run/signing.keychain-db")).toEqual([
    "xcrun", "notarytool", "store-credentials", "lightcal-ci", "--key", "/tmp/run/key.p8",
    "--key-id", "ABCDE12345", "--issuer", "69a6de7e-1234-47e3-e053-5b8c7c11a4d1",
    "--keychain", "/tmp/run/signing.keychain-db",
  ]);
  expect(key.privateKey).toBe(`${pem}\n`);
});

test("the .p8 secret may be PEM text or base64 of it", () => {
  expect(notaryApiKey(apiEnv({ APPLE_API_KEY_P8: Buffer.from(pem).toString("base64") })).privateKey).toBe(`${pem}\n`);
  expect(notaryApiKey(apiEnv({ APPLE_API_KEY_P8: `\n${pem}\n\n` })).privateKey).toBe(`${pem}\n`);
});

test("malformed API key settings fail without echoing their values", () => {
  const cases: [Record<string, string | undefined>, RegExp][] = [
    [{ APPLE_API_KEY: "short" }, /APPLE_API_KEY must be/],
    [{ APPLE_API_KEY: undefined }, /APPLE_API_KEY must be/],
    [{ APPLE_API_ISSUER: "not-a-uuid" }, /APPLE_API_ISSUER must be/],
    [{ APPLE_API_KEY_P8: "/Users/someone/AuthKey_ABCDE12345.p8" }, /APPLE_API_KEY_P8 must contain/],
    [{ APPLE_API_KEY_P8: "" }, /APPLE_API_KEY_P8 must contain/],
  ];
  for (const [overrides, message] of cases) {
    let error: unknown;
    try { notaryApiKey(apiEnv(overrides)); } catch (caught) { error = caught; }
    expect(String(error)).toMatch(message);
    for (const value of Object.values(overrides)) if (value) expect(String(error)).not.toContain(value);
  }
});
