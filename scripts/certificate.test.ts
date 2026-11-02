import { describe, expect, test } from "bun:test";
import { normalizeFingerprint, pinListIncludes } from "./certificate.ts";

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
