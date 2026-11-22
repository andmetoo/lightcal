/** Detect whether macOS signing, notarization and Sparkle release configuration is complete. */
import { appendFile } from "node:fs/promises";

export const MACOS_RELEASE_CONFIG_KEYS = [
  "APPLE_SIGNING_IDENTITY",
  "APPLE_CERTIFICATE_P12",
  "APPLE_CERTIFICATE_PASSWORD",
  "APPLE_API_KEY",
  "APPLE_API_ISSUER",
  "APPLE_API_KEY_P8",
  "SPARKLE_SHA256",
  "SPARKLE_PUBLIC_KEY",
  "SPARKLE_PRIVATE_KEY",
  "LIGHTCAL_MICROSOFT_CLIENT_ID",
  "LIGHTCAL_GOOGLE_CLIENT_ID",
  "LIGHTCAL_GOOGLE_CLIENT_SECRET",
] as const;

export function missingMacSigningConfig(env: Record<string, string | undefined>): string[] {
  return MACOS_RELEASE_CONFIG_KEYS.filter(key => !env[key]?.trim());
}

/** Release-only signing material; the client IDs are shared with every build. */
export const MACOS_SIGNING_KEYS = MACOS_RELEASE_CONFIG_KEYS.filter(key => !key.startsWith("LIGHTCAL_"));

/**
 * "absent": no signing material at all, so the release job keeps the unsigned fallback.
 * "partial": some signing values are set but others are missing or empty (for example a
 * secret saved from an empty pipe). That is a broken release setup, and the job fails.
 */
export function macSigningConfigState(env: Record<string, string | undefined>): "complete" | "partial" | "absent" {
  const missing = missingMacSigningConfig(env);
  if (missing.length === 0) return "complete";
  return MACOS_SIGNING_KEYS.every(key => missing.includes(key)) ? "absent" : "partial";
}

if (import.meta.main) {
  const missing = missingMacSigningConfig(process.env);
  const state = macSigningConfigState(process.env);
  const configured = state === "complete";
  if (process.env.GITHUB_OUTPUT) {
    await appendFile(
      process.env.GITHUB_OUTPUT,
      `configured=${configured}\nmissing=${missing.join(",")}\n`,
    );
  }
  if (configured) {
    console.log("macOS signing, notarization and Sparkle configuration is complete.");
  } else if (state === "partial") {
    const message = `macOS release signing is partly configured; missing or empty: ${missing.join(", ")}. ` +
      "Set these in the macos-release environment (or remove all signing values to build unsigned) and rerun.";
    console.log(process.env.GITHUB_ACTIONS === "true" ? `::error::${message}` : message);
    process.exit(1);
  } else {
    const list = missing.join(", ");
    const message =
      `macOS signing configuration is incomplete (missing: ${list}). Falling back to building an unsigned universal DMG artifact; nothing will be attached to the GitHub Release.`;
    console.log(process.env.GITHUB_ACTIONS === "true" ? `::warning::${message}` : message);
    if (process.env.GITHUB_STEP_SUMMARY) {
      await appendFile(
        process.env.GITHUB_STEP_SUMMARY,
        `### Unsigned macOS DMG fallback\n\nSigning/notarization configuration is incomplete (missing: ${missing.map(key => `\`${key}\``).join(", ")}).\n\nFalling back to building an unsigned universal DMG into workflow artifact \`lightcal-macos-universal-unsigned\`. No assets will be attached to the GitHub Release.\n`,
      );
    }
  }
}
