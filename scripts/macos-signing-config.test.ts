import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { MACOS_RELEASE_CONFIG_KEYS, MACOS_SIGNING_KEYS, macSigningConfigState, missingMacSigningConfig } from "./macos-signing-config";
import { NOTARY_API_KEY_ENV } from "./macos-notarization";

test("missingMacSigningConfig returns empty list when all keys are set", () => {
  const env = Object.fromEntries(MACOS_RELEASE_CONFIG_KEYS.map(key => [key, "configured"]));
  expect(missingMacSigningConfig(env)).toEqual([]);
});

test("missingMacSigningConfig reports absent, empty and whitespace-only keys", () => {
  const env: Record<string, string | undefined> = Object.fromEntries(
    MACOS_RELEASE_CONFIG_KEYS.map(key => [key, "configured"]),
  );
  delete env.APPLE_CERTIFICATE_P12;
  env.APPLE_SIGNING_IDENTITY = "";
  env.SPARKLE_SHA256 = "   ";

  expect(missingMacSigningConfig(env)).toEqual([
    "APPLE_SIGNING_IDENTITY",
    "APPLE_CERTIFICATE_P12",
    "SPARKLE_SHA256",
  ]);
});

test("missingMacSigningConfig reports all keys when env is empty", () => {
  expect(missingMacSigningConfig({})).toEqual([...MACOS_RELEASE_CONFIG_KEYS]);
});

test("MACOS_RELEASE_CONFIG_KEYS covers every external key required by signing and packaging scripts", async () => {
  const [signingScript, packageScript] = await Promise.all([
    readFile(join(import.meta.dir, "prepare-macos-signing.ts"), "utf8"),
    readFile(join(import.meta.dir, "package-macos.ts"), "utf8"),
  ]);
  const workflowGenerated = new Set([
    "APPLE_NOTARY_PROFILE",
    "APPLE_SIGNING_KEYCHAIN",
    "SPARKLE_DIR",
    "SPARKLE_PRIVATE_KEY_FILE",
    "GITHUB_REPOSITORY",
  ]);
  const signingRequired = [...signingScript.matchAll(/required\("([A-Z0-9_]+)"\)/g)].map(m => m[1]);
  const packageListMatch = packageScript.match(/if \(release\) for \(const key of \[([^\]]+)\]\)/);
  expect(packageListMatch).not.toBeNull();
  const packageRequired = [...packageListMatch![1].matchAll(/"([A-Z0-9_]+)"/g)].map(m => m[1]);

  const expectedExternalKeys = new Set(
    [...signingRequired, ...packageRequired].filter(key => !workflowGenerated.has(key)),
  );
  const configuredKeys = new Set<string>(MACOS_RELEASE_CONFIG_KEYS);
  for (const key of expectedExternalKeys) {
    expect(configuredKeys.has(key), `missing ${key} in MACOS_RELEASE_CONFIG_KEYS`).toBe(true);
  }
});

test("partial signing configuration is an error, while none at all keeps the unsigned fallback", () => {
  const complete = Object.fromEntries(MACOS_RELEASE_CONFIG_KEYS.map(key => [key, "configured"]));
  expect(macSigningConfigState(complete)).toBe("complete");
  // An empty secret, as `gh secret set` stores from an empty pipe, counts as missing.
  expect(macSigningConfigState({ ...complete, APPLE_CERTIFICATE_P12: "" })).toBe("partial");
  expect(macSigningConfigState({ ...complete, SPARKLE_SHA256: "  " })).toBe("partial");
  // Only the shared client IDs: a repository that never set up macOS signing.
  const clientIds = Object.fromEntries(MACOS_RELEASE_CONFIG_KEYS.filter(key => key.startsWith("LIGHTCAL_")).map(key => [key, "configured"]));
  expect(macSigningConfigState(clientIds)).toBe("absent");
  expect(macSigningConfigState({})).toBe("absent");
  expect(MACOS_SIGNING_KEYS).not.toContain("LIGHTCAL_GOOGLE_CLIENT_ID");
});

test("the notarization API key names are part of the release configuration", async () => {
  for (const key of NOTARY_API_KEY_ENV) expect(MACOS_RELEASE_CONFIG_KEYS as readonly string[]).toContain(key);
  // notaryApiKey must read its variables through NOTARY_API_KEY_ENV, not hard-coded names.
  const source = await readFile(join(import.meta.dir, "macos-notarization.ts"), "utf8");
  const body = source.slice(source.indexOf("export function notaryApiKey"), source.indexOf("export function notaryStoreCredentialsCommand"));
  expect(body).toContain("NOTARY_API_KEY_ENV");
  expect(body).not.toMatch(/env\.APPLE_/);
});
