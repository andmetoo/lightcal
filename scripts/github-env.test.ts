import { describe, expect, test } from "bun:test";
import { ENTRIES, ghArgs, parseDotenv, plan } from "./github-env.ts";

const GOOGLE_ID = "1234567890-abcdef.apps.googleusercontent.com";
const MS_ID = "00000000-1111-2222-3333-444444444444";
const PIN = "a".repeat(64);

describe("github-env", () => {
  test("parses dotenv like build.rs", () => {
    const values = parseDotenv(`# comment\nLIGHTCAL_GOOGLE_CLIENT_ID="${GOOGLE_ID}"\r\nEMPTY=\nX = 'y'\nX=second\n`);
    expect(values.get("LIGHTCAL_GOOGLE_CLIENT_ID")).toBe(GOOGLE_ID);
    expect(values.get("EMPTY")).toBe("");
    expect(values.get("X")).toBe("y");
  });

  test("process environment wins and empty values are skipped", () => {
    const dotenv = parseDotenv(`LIGHTCAL_MICROSOFT_CLIENT_ID=${MS_ID}\nLIGHTCAL_GOOGLE_CLIENT_ID=old.apps.googleusercontent.com\n`);
    const result = plan({ LIGHTCAL_GOOGLE_CLIENT_ID: GOOGLE_ID, LIGHTCAL_UPDATE_CERT_SHA256: "" }, dotenv);
    const byKey = Object.fromEntries(result.map((p) => [p.entry.key, p]));
    expect(byKey.LIGHTCAL_GOOGLE_CLIENT_ID.value).toBe(GOOGLE_ID);
    expect(byKey.LIGHTCAL_MICROSOFT_CLIENT_ID.value).toBe(MS_ID);
    expect(byKey.LIGHTCAL_UPDATE_CERT_SHA256.value).toBeUndefined();
    expect(result.every((p) => !p.error)).toBe(true);
  });

  test("rejects malformed values", () => {
    const result = plan(
      { LIGHTCAL_MICROSOFT_CLIENT_ID: "nope", LIGHTCAL_UPDATE_CERT_SHA256: `${PIN},bad`, LIGHTCAL_GOOGLE_CLIENT_SECRET: "short" },
      new Map(),
    );
    expect(result.filter((p) => p.error).map((p) => p.entry.key)).toEqual([
      "LIGHTCAL_MICROSOFT_CLIENT_ID",
      "LIGHTCAL_UPDATE_CERT_SHA256",
      "LIGHTCAL_GOOGLE_CLIENT_SECRET",
    ]);
  });

  test("secrets never go on the command line", () => {
    const secret = ENTRIES.find((e) => e.kind === "secret")!;
    expect(ghArgs(secret, "o/r", "GOCSPX-value")).not.toContain("GOCSPX-value");
    const variable = ENTRIES.find((e) => e.key === "LIGHTCAL_GOOGLE_CLIENT_ID")!;
    expect(ghArgs(variable, "o/r", GOOGLE_ID)).toEqual(["gh", "variable", "set", variable.key, "--repo", "o/r", "--body", GOOGLE_ID]);
  });
});
