import { describe, expect, test } from "bun:test";
import {
  ENTRIES,
  ghArgs,
  parseArgs,
  parseDotenv,
  pinMismatch,
  plan,
  resolvePinCheck,
  validCertificatePins,
  validGoogleClientId,
  validGoogleClientSecret,
  validMicrosoftClientId,
} from "./github-env.ts";

const GOOGLE_ID = "1234567890-abcdef.apps.googleusercontent.com";
const MS_ID = "00000000-1111-2222-3333-444444444444";
const PIN = "a".repeat(64);

// Cases taken from the Rust validators in crates/lightcal/src (google.rs, microsoft.rs,
// installer.rs). Keep both sides in step when either changes.
describe("validators mirror the app", () => {
  test("Google client ID", () => {
    expect(validGoogleClientId(GOOGLE_ID)).toBe(true);
    expect(validGoogleClientId("1-a.apps.googleusercontent.com")).toBe(false); // prefix too short
    expect(validGoogleClientId("1234567890abcdef.apps.googleusercontent.com")).toBe(false); // no '-'
    expect(validGoogleClientId(" 1234567890-abcdef.apps.googleusercontent.com")).toBe(false);
    expect(validGoogleClientId("1234567890-abc.def.apps.googleusercontent.com")).toBe(false);
  });

  test("Google client secret", () => {
    expect(validGoogleClientSecret("GOCSPX-abcdefghij123456")).toBe(true);
    expect(validGoogleClientSecret("abcdefghij")).toBe(true); // 10 is the minimum
    expect(validGoogleClientSecret("short")).toBe(false);
    expect(validGoogleClientSecret('GOCSPX-abc"defghij')).toBe(false);
    expect(validGoogleClientSecret("GOCSPX-abcdéfghij")).toBe(false);
    expect(validGoogleClientSecret("x".repeat(256))).toBe(false);
  });

  test("Microsoft client ID", () => {
    expect(validMicrosoftClientId(MS_ID)).toBe(true);
    expect(validMicrosoftClientId(`{${MS_ID}}`)).toBe(false);
  });

  test("certificate pins accept spaces and colons like normalize_pin", () => {
    expect(validCertificatePins(`${PIN},${"b".repeat(64)}`)).toBe(true);
    expect(validCertificatePins("AB ".repeat(32).trim())).toBe(true);
    expect(validCertificatePins("ab:".repeat(31) + "ab")).toBe(true);
    expect(validCertificatePins(`${PIN},bad`)).toBe(false);
    expect(validCertificatePins("")).toBe(false);
  });
});

describe("github-env", () => {
  test("parses dotenv like build.rs", () => {
    const values = parseDotenv(`  # comment\nLIGHTCAL_GOOGLE_CLIENT_ID="${GOOGLE_ID}"\r\nEMPTY=\nX = ''y''\nX=second\n`);
    expect(values.get("LIGHTCAL_GOOGLE_CLIENT_ID")).toBe(GOOGLE_ID);
    expect(values.get("EMPTY")).toBe("");
    expect(values.get("X")).toBe("y");
  });

  test("process environment wins, even when empty", () => {
    const dotenv = parseDotenv(
      `LIGHTCAL_MICROSOFT_CLIENT_ID=${MS_ID}\nLIGHTCAL_GOOGLE_CLIENT_ID=old-1234567890.apps.googleusercontent.com\nLIGHTCAL_UPDATE_CERT_SHA256=${PIN}\n`,
    );
    const result = plan({ LIGHTCAL_GOOGLE_CLIENT_ID: GOOGLE_ID, LIGHTCAL_UPDATE_CERT_SHA256: "" }, dotenv);
    const byKey = Object.fromEntries(result.map((p) => [p.entry.key, p]));
    expect(byKey.LIGHTCAL_GOOGLE_CLIENT_ID.value).toBe(GOOGLE_ID);
    expect(byKey.LIGHTCAL_MICROSOFT_CLIENT_ID.value).toBe(MS_ID);
    expect(byKey.LIGHTCAL_UPDATE_CERT_SHA256.value).toBeUndefined();
    expect(result.every((p) => !p.error)).toBe(true);
  });

  test("rejects values the app would reject", () => {
    const result = plan(
      {
        LIGHTCAL_MICROSOFT_CLIENT_ID: "nope",
        LIGHTCAL_GOOGLE_CLIENT_ID: "1-a.apps.googleusercontent.com",
        LIGHTCAL_UPDATE_CERT_SHA256: `${PIN},bad`,
        LIGHTCAL_GOOGLE_CLIENT_SECRET: "short",
      },
      new Map(),
    );
    expect(result.filter((p) => p.error).map((p) => p.entry.key)).toEqual(ENTRIES.map((e) => e.key));
  });

  test("parses arguments strictly", () => {
    expect(parseArgs([])).toEqual({ dryRun: false, skipPinCheck: false });
    expect(parseArgs(["--repo", "o/r", "--dry-run"])).toEqual({ dryRun: true, skipPinCheck: false, repo: "o/r" });
    expect(parseArgs(["--skip-pin-check"])).toEqual({ dryRun: false, skipPinCheck: true });
    expect(() => parseArgs(["--repo", "--dry-run"])).toThrow(/owner\/name/);
    expect(() => parseArgs(["--repo"])).toThrow(/owner\/name/);
    expect(() => parseArgs(["--bogus"])).toThrow(/Unknown argument/);
  });

  test("secrets never go on the command line", () => {
    const secret = ENTRIES.find((e) => e.kind === "secret")!;
    expect(ghArgs(secret, "o/r", "GOCSPX-value")).not.toContain("GOCSPX-value");
    const variable = ENTRIES.find((e) => e.key === "LIGHTCAL_GOOGLE_CLIENT_ID")!;
    expect(ghArgs(variable, "o/r", GOOGLE_ID)).toEqual(["gh", "variable", "set", variable.key, "--repo", "o/r", "--body", GOOGLE_ID]);
  });
});

describe("pin against the signing certificate", () => {
  const cert = "538EFF7282EC191B33B46E26DAA09AE756F022BAE467D2736ACA01F69DD9CAD6";
  test("refuses a pin that leaves out the signing certificate", () => {
    const fileChecksum = "f37b579af600613e4b2c54a89ab4395d1e1ad983ecd573e0ad33b49e3e56e678";
    expect(pinMismatch(fileChecksum, cert)).toContain(cert);
  });
  test("accepts the certificate, also inside a renewal list", () => {
    expect(pinMismatch(cert.toLowerCase(), cert)).toBeUndefined();
    expect(pinMismatch(`${"a".repeat(64)},${cert}`, cert)).toBeUndefined();
  });
  test("says nothing without a pin or a certificate to compare", () => {
    expect(pinMismatch(undefined, cert)).toBeUndefined();
    expect(pinMismatch(cert, null)).toBeUndefined();
  });
});

describe("pin check inside plan", () => {
  const cert = "538EFF7282EC191B33B46E26DAA09AE756F022BAE467D2736ACA01F69DD9CAD6";
  const wrong = "f37b579af600613e4b2c54a89ab4395d1e1ad983ecd573e0ad33b49e3e56e678";
  const pinError = (pins: string, context = {}) =>
    plan({ LIGHTCAL_UPDATE_CERT_SHA256: pins }, new Map(), context)
      .find((p) => p.entry.key === "LIGHTCAL_UPDATE_CERT_SHA256")?.error;
  test("compares the pin with the signing certificate when one was read", () => {
    expect(pinError(wrong, { signingCertSha256: cert })).toContain(cert);
    expect(pinError(cert, { signingCertSha256: cert })).toBeUndefined();
  });
  test("checks only the format without a certificate", () => {
    expect(pinError(wrong)).toBeUndefined();
    expect(pinError("not-a-pin", { signingCertSha256: cert })).toMatch(/64-hex/);
  });
});

describe("resolving the signing-certificate lookup", () => {
  const cert = "538EFF7282EC191B33B46E26DAA09AE756F022BAE467D2736ACA01F69DD9CAD6";
  test("a found certificate becomes the check context", () => {
    expect(resolvePinCheck({ kind: "found", sha256: cert }, false)).toEqual({ context: { signingCertSha256: cert } });
  });
  test("signed-out SimplySign or a broken lookup stops the sync unless skipped", () => {
    expect(resolvePinCheck({ kind: "absent" }, false).error).toMatch(/SimplySign.*--skip-pin-check/);
    expect(resolvePinCheck({ kind: "failed", reason: "pwsh missing" }, false).error).toMatch(/pwsh missing.*--skip-pin-check/);
    expect(resolvePinCheck({ kind: "absent" }, true)).toEqual({
      context: {},
      note: "LIGHTCAL_UPDATE_CERT_SHA256 not compared with the signing certificate (--skip-pin-check)",
    });
  });
  test("no lookup possible (Linux, no SIGN_CERT_SHA1) only adds a note", () => {
    const resolved = resolvePinCheck({ kind: "unsupported", reason: "SIGN_CERT_SHA1 is not set" }, false);
    expect(resolved.error).toBeUndefined();
    expect(resolved.note).toContain("SIGN_CERT_SHA1 is not set");
  });
});
