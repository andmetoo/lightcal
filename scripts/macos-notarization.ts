/** Both submissions use the profile stored in the ephemeral signing keychain. */
export function notarySubmitCommand(artifact: string, profile: string, keychain: string): string[] {
  return ["xcrun", "notarytool", "submit", artifact, "--keychain-profile", profile,
    "--keychain", keychain, "--wait"];
}

/** App Store Connect team API key used for notarization (no Apple ID or app password). */
export type NotaryApiKey = { keyId: string; issuer: string; privateKey: string };

const KEY_ID = /^[A-Z0-9]{8,32}$/;
const ISSUER = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const PEM = /-----BEGIN PRIVATE KEY-----[\s\S]+-----END PRIVATE KEY-----/;

/**
 * Validates the API key settings without echoing them. APPLE_API_KEY_P8 may be the
 * .p8 file's PEM text or that text base64-encoded.
 */
export function notaryApiKey(env: Record<string, string | undefined>): NotaryApiKey {
  const keyId = env.APPLE_API_KEY?.trim() ?? "";
  const issuer = env.APPLE_API_ISSUER?.trim() ?? "";
  const raw = env.APPLE_API_KEY_P8?.trim() ?? "";
  if (!KEY_ID.test(keyId)) throw new Error("APPLE_API_KEY must be the App Store Connect key ID (letters and digits)");
  if (!ISSUER.test(issuer)) throw new Error("APPLE_API_ISSUER must be the App Store Connect issuer ID (a UUID)");
  const pem = PEM.test(raw) ? raw : Buffer.from(raw, "base64").toString("utf8").trim();
  if (!PEM.test(pem)) throw new Error("APPLE_API_KEY_P8 must contain the AuthKey .p8 private key (PEM text or base64)");
  return { keyId, issuer, privateKey: `${pem}\n` };
}

/** Stores a notarytool profile for the API key in the temporary signing keychain. */
export function notaryStoreCredentialsCommand(profile: string, keyFile: string, key: NotaryApiKey, keychain: string): string[] {
  return ["xcrun", "notarytool", "store-credentials", profile, "--key", keyFile, "--key-id", key.keyId,
    "--issuer", key.issuer, "--keychain", keychain];
}
