import { describe, expect, test } from "bun:test";
import {
  ENTRIES,
  ghArgs,
  parseArgs,
  parseDotenv,
  plan,
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
    expect(parseArgs([])).toEqual({ dryRun: false });
    expect(parseArgs(["--repo", "o/r", "--dry-run"])).toEqual({ dryRun: true, repo: "o/r" });
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
