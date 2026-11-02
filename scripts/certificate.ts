/**
 * Code-signing certificate fingerprints, shared by the release tooling and github-env.
 *
 * The app's update pin (LIGHTCAL_UPDATE_CERT_SHA256) is the SHA-256 of the signing
 * certificate's raw bytes. These helpers compare it the way `installer::normalize_pin` does:
 * spaces and colons ignored, any letter case, comma-separated lists allowed.
 */

/** Strips spaces and colons and lowercases, as the app normalizes a pin. */
export function normalizeFingerprint(raw: string): string {
  return raw.replace(/[ :]/g, "").toLowerCase();
}

/** Whether a comma-separated pin list accepts the certificate with this SHA-256. */
export function pinListIncludes(pins: string, certSha256: string): boolean {
  const cert = normalizeFingerprint(certSha256);
  return pins.split(",").some((pin) => normalizeFingerprint(pin) === cert);
}

/**
 * SHA-256 of the certificate with this SHA-1 thumbprint in the current-user store, or null
 * when not on Windows or the certificate is not there (SimplySign Desktop signed out).
 */
export function storeCertificateSha256(thumbprint: string): string | null {
  if (process.platform !== "win32") return null;
  const thumb = thumbprint.replace(/\s/g, "").toUpperCase();
  if (!/^[0-9A-F]{40}$/.test(thumb)) return null;
  const proc = Bun.spawnSync(["pwsh", "-NoLogo", "-NoProfile", "-Command",
    `$c = Get-Item -LiteralPath 'Cert:\\CurrentUser\\My\\${thumb}' -ErrorAction SilentlyContinue; ` +
    "if ($c) { [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($c.RawData)) }",
  ], { stdout: "pipe", stderr: "pipe" });
  const last = proc.stdout.toString().trim().split(/\r?\n/).at(-1)?.trim() ?? "";
  return /^[0-9A-F]{64}$/i.test(last) ? last.toUpperCase() : null;
}
