/** Detect whether macOS signing, notarization and Sparkle release configuration is complete. */
import { appendFile } from "node:fs/promises";

export const MACOS_RELEASE_CONFIG_KEYS = [
  "APPLE_SIGNING_IDENTITY",
  "APPLE_CERTIFICATE_P12",
  "APPLE_CERTIFICATE_PASSWORD",
  "APPLE_ID",
  "APPLE_TEAM_ID",
  "APPLE_APP_PASSWORD",
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

if (import.meta.main) {
  const missing = missingMacSigningConfig(process.env);
  const configured = missing.length === 0;
  if (process.env.GITHUB_OUTPUT) {
    await appendFile(
      process.env.GITHUB_OUTPUT,
      `configured=${configured}\nmissing=${missing.join(",")}\n`,
    );
  }
  if (configured) {
    console.log("macOS signing, notarization and Sparkle configuration is complete.");
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
