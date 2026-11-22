import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { MACOS_RELEASE_CONFIG_KEYS, missingMacSigningConfig } from "./macos-signing-config";

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
