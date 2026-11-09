/** Both submissions use the profile stored in the ephemeral signing keychain. */
export function notarySubmitCommand(artifact: string, profile: string, keychain: string): string[] {
  return ["xcrun", "notarytool", "submit", artifact, "--keychain-profile", profile,
    "--keychain", keychain, "--wait"];
}
