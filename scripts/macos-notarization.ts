/** Both submissions use the profile stored in the ephemeral signing keychain. */
export function notarySubmitCommand(artifact: string, profile: string, keychain: string): string[] {
  return ["xcrun", "notarytool", "submit", artifact, "--keychain-profile", profile,
    "--keychain", keychain, "--wait"];
}

/** App Store Connect team API key used for notarization (no Apple ID or app password). */
export type NotaryApiKey = { keyId: string; issuer: string; privateKey: string };

/** Environment names read by notaryApiKey; the release config check lists the same names. */
export const NOTARY_API_KEY_ENV = ["APPLE_API_KEY", "APPLE_API_ISSUER", "APPLE_API_KEY_P8"] as const;

const KEY_ID = /^[A-Z0-9]{8,32}$/;
const ISSUER = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
// Real line breaks around the base64 body: a one-line value with literal "\n" is not a usable .p8.
const PEM = /^-----BEGIN PRIVATE KEY-----\r?\n[A-Za-z0-9+/=\r\n]+\r?\n-----END PRIVATE KEY-----$/;

/**
 * Validates the API key settings without echoing them. APPLE_API_KEY_P8 may be the
 * .p8 file's PEM text or that text base64-encoded.
 */
export function notaryApiKey(env: Record<string, string | undefined>): NotaryApiKey {
  const [keyIdName, issuerName, privateKeyName] = NOTARY_API_KEY_ENV;
  const keyId = env[keyIdName]?.trim() ?? "";
  const issuer = env[issuerName]?.trim() ?? "";
  const raw = env[privateKeyName]?.trim() ?? "";
  if (!KEY_ID.test(keyId)) throw new Error("APPLE_API_KEY must be the App Store Connect key ID (letters and digits)");
  if (!ISSUER.test(issuer)) throw new Error("APPLE_API_ISSUER must be the App Store Connect issuer ID (a UUID)");
  if (raw.includes("\\n") && !raw.includes("\n")) {
    throw new Error("APPLE_API_KEY_P8 contains literal \"\\n\" sequences; store the .p8 file contents with real line breaks");
  }
  const pem = raw.startsWith("-----BEGIN") ? raw : Buffer.from(raw, "base64").toString("utf8").trim();
  if (!PEM.test(pem)) throw new Error("APPLE_API_KEY_P8 must contain the AuthKey .p8 private key (PEM text or base64)");
  return { keyId, issuer, privateKey: `${pem}\n` };
}

/** Stores a notarytool profile for the API key in the temporary signing keychain. */
export function notaryStoreCredentialsCommand(profile: string, keyFile: string, key: NotaryApiKey, keychain: string): string[] {
  return ["xcrun", "notarytool", "store-credentials", profile, "--key", keyFile, "--key-id", key.keyId,
    "--issuer", key.issuer, "--keychain", keychain];
}

/**
 * Apple's reason for rejecting the API key (wrong issuer, revoked key, a role that
 * cannot notarize), without the key itself: notarytool never prints it, and GitHub
 * masks secret values in logs.
 */
export function notaryFailureMessage(stderr: string): string {
  const detail = stderr.split(/\r?\n/).map((line) => line.trim()).filter(Boolean).slice(-5).join(" | ");
  return `notarytool store-credentials failed; check APPLE_API_KEY, APPLE_API_ISSUER and APPLE_API_KEY_P8 ` +
    `(a Team key with the Developer role or higher)${detail ? `: ${detail}` : ""}`;
}
