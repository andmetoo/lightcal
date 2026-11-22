import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { DMG_LAYOUT, dmgBackgroundSvg, finderLayoutScript } from "./macos-dmg";

test("Finder places both icons where the background expects them", () => {
  const script = finderLayoutScript();
  expect(script).toContain(`tell disk "${DMG_LAYOUT.volumeName}"`);
  expect(script).toContain(`set position of item "Lightcal.app" of container window to {${DMG_LAYOUT.app.x}, ${DMG_LAYOUT.app.y}}`);
  expect(script).toContain(`set position of item "Applications" of container window to {${DMG_LAYOUT.applications.x}, ${DMG_LAYOUT.applications.y}}`);
  expect(script).toContain(`set icon size of viewOptions to ${DMG_LAYOUT.iconSize}`);
  expect(script).toContain('set background picture of viewOptions to file ".background:background.tiff"');
  expect(script).toContain("set toolbar visible of container window to false");
  expect(script).toContain("set pathbar visible of container window to false");
  // The window's content area matches the background size; the title bar is added on top.
  expect(script).toMatch(new RegExp(`\\{200, 120, ${200 + DMG_LAYOUT.width}, ${120 + DMG_LAYOUT.height + 28}\\}`));
});

test("AppleScript strings are quoted", () => {
  const script = finderLayoutScript({ ...DMG_LAYOUT, volumeName: 'Light "cal"' });
  expect(script).toContain('tell disk "Light \\"cal\\""');
});

test("the background keeps the label row clear and the footer above a path bar", () => {
  const svg = dmgBackgroundSvg();
  // Finder draws the icon names; nothing in the art may sit behind them.
  const labelTop = DMG_LAYOUT.app.y + DMG_LAYOUT.iconSize / 2;
  const footerY = Number(svg.match(/<text x="[\d.]+" y="([\d.]+)" text-anchor="middle"/)?.[1]);
  expect(footerY).toBeLessThanOrEqual(DMG_LAYOUT.height - 34);
  expect(footerY).toBeGreaterThan(labelTop + 40);
  expect(svg).not.toContain('height="20" rx="10"');
  expect(dmgBackgroundSvg(DMG_LAYOUT, 2)).toContain(`width="${DMG_LAYOUT.width * 2}" height="${DMG_LAYOUT.height * 2}" viewBox="0 0 ${DMG_LAYOUT.width} ${DMG_LAYOUT.height}"`);
});

test("the committed background images match the layout size", () => {
  for (const [name, scale] of [["dmg-background.png", 1], ["dmg-background@2x.png", 2]] as const) {
    const path = join(import.meta.dir, "../assets/macos", name);
    expect(existsSync(path)).toBe(true);
    const png = readFileSync(path);
    // PNG IHDR: width and height are big-endian at bytes 16 and 20.
    expect([png.readUInt32BE(16), png.readUInt32BE(20)]).toEqual([DMG_LAYOUT.width * scale, DMG_LAYOUT.height * scale]);
  }
});
