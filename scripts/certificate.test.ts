import { describe, expect, test } from "bun:test";
import { type Spawn, lookupStoreCertificate, normalizeFingerprint, pinListIncludes } from "./certificate.ts";

const cert = "538EFF7282EC191B33B46E26DAA09AE756F022BAE467D2736ACA01F69DD9CAD6";

describe("certificate pin lists", () => {
  test("normalize like the app: spaces and colons ignored, any case", () => {
    expect(normalizeFingerprint("53:8E ff")).toBe("538eff");
  });
  test("accept the certificate in any spelling and inside a renewal list", () => {
    expect(pinListIncludes(cert.toLowerCase(), cert)).toBe(true);
    expect(pinListIncludes(cert.match(/../g)!.join(" "), cert)).toBe(true);
    expect(pinListIncludes(`${"a".repeat(64)},${cert.match(/../g)!.join(":")}`, cert)).toBe(true);
  });
  test("reject a list without the certificate", () => {
    expect(pinListIncludes("f37b579af600613e4b2c54a89ab4395d1e1ad983ecd573e0ad33b49e3e56e678", cert)).toBe(false);
  });
});

describe("store certificate lookup", () => {
  const thumb = "F0BBA075C11AFBD3E2798BF517A8F4C3CE74AF6B";
  const replying = (exitCode: number, stdout: string, stderr = ""): Spawn => () => ({ exitCode, stdout, stderr });
  test("returns the certificate hash", () => {
    expect(lookupStoreCertificate(thumb, "win32", replying(0, `warning\n${cert.toLowerCase()}\n`)))
      .toEqual({ kind: "found", sha256: cert });
  });
  test("reports an absent certificate separately from a failed lookup", () => {
    expect(lookupStoreCertificate(thumb, "win32", replying(0, ""))).toEqual({ kind: "absent" });
    expect(lookupStoreCertificate(thumb, "win32", replying(1, "", "Get-Item: access denied")))
      .toEqual({ kind: "failed", reason: "Get-Item: access denied" });
    expect(lookupStoreCertificate(thumb, "win32", replying(0, "not a hash")).kind).toBe("failed");
  });
  test("a missing PowerShell 7 is a failed lookup, not a crash", () => {
    const missing: Spawn = () => { throw new Error('Executable not found in $PATH: "pwsh"'); };
    const result = lookupStoreCertificate(thumb, "win32", missing);
    expect(result.kind).toBe("failed");
    expect(result.kind === "failed" && result.reason).toContain("pwsh");
  });
  test("no lookup off Windows or without a thumbprint", () => {
    expect(lookupStoreCertificate(thumb, "linux", replying(0, cert)).kind).toBe("unsupported");
    expect(lookupStoreCertificate("", "win32", replying(0, cert)).kind).toBe("unsupported");
    expect(lookupStoreCertificate("1234", "win32", replying(0, cert)).kind).toBe("failed");
  });
});
