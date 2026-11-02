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
 * Outcome of looking up the signing certificate in the current-user store:
 * - `found`: its SHA-256;
 * - `absent`: the store has no such certificate (SimplySign Desktop signed out);
 * - `unsupported`: no lookup is possible here (not Windows, or no SIGN_CERT_SHA1);
 * - `failed`: the lookup itself broke (PowerShell 7 missing, a store error).
 */
export type CertificateLookup =
  | { kind: "found"; sha256: string }
  | { kind: "absent" }
  | { kind: "unsupported"; reason: string }
  | { kind: "failed"; reason: string };

export type Spawn = (cmd: string[]) => { exitCode: number; stdout: string; stderr: string };

const spawnPowerShell: Spawn = (cmd) => {
  const proc = Bun.spawnSync(cmd, { stdout: "pipe", stderr: "pipe" });
  return { exitCode: proc.exitCode, stdout: proc.stdout.toString(), stderr: proc.stderr.toString() };
};

/** Finds the certificate with this SHA-1 thumbprint in the current-user store. */
export function lookupStoreCertificate(
  thumbprint: string,
  platform: string = process.platform,
  spawn: Spawn = spawnPowerShell,
): CertificateLookup {
  if (platform !== "win32") return { kind: "unsupported", reason: "the certificate store is only readable on Windows" };
  const thumb = thumbprint.replace(/\s/g, "").toUpperCase();
  if (!thumb) return { kind: "unsupported", reason: "SIGN_CERT_SHA1 is not set" };
  if (!/^[0-9A-F]{40}$/.test(thumb)) return { kind: "failed", reason: "SIGN_CERT_SHA1 is not a 40-character SHA-1 thumbprint" };
  let result: ReturnType<Spawn>;
  try {
    // A missing certificate prints nothing and exits 0; any other error exits 1 with a message.
    result = spawn(["pwsh", "-NoLogo", "-NoProfile", "-Command",
      "$ErrorActionPreference = 'Stop'; try { " +
      `$c = Get-Item -LiteralPath 'Cert:\\CurrentUser\\My\\${thumb}' -ErrorAction SilentlyContinue; ` +
      "if ($c) { [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($c.RawData)) }; exit 0 " +
      "} catch { [Console]::Error.WriteLine($_); exit 1 }",
    ]);
  } catch (error) {
    return { kind: "failed", reason: `could not start PowerShell 7 (pwsh): ${error instanceof Error ? error.message : error}` };
  }
  if (result.exitCode !== 0) {
    return { kind: "failed", reason: result.stderr.trim() || `pwsh exited with code ${result.exitCode}` };
  }
  const last = result.stdout.trim().split(/\r?\n/).at(-1)?.trim() ?? "";
  if (!last) return { kind: "absent" };
  if (!/^[0-9A-F]{64}$/i.test(last)) return { kind: "failed", reason: `unexpected certificate hash output: ${last}` };
  return { kind: "found", sha256: last.toUpperCase() };
}
