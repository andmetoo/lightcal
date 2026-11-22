// Render the macOS disk image background to assets/macos/dmg-background.png and @2x.
//
// The art is generated from DMG_LAYOUT (scripts/macos-dmg.ts) so it matches the Finder view,
// and rasterized by resvg, a development-only tool that is never committed:
//
//   bun add --cwd target/design-tools @resvg/resvg-js@2.6.2
//   bun scripts/render-dmg-background.ts [--svg target/dmg-background.svg]
//
// macOS only: the text uses Helvetica Neue from /System/Library/Fonts. Fonts are never
// copied into the repository. scripts/package-macos.ts combines both PNGs into one
// HiDPI TIFF with tiffutil when it builds the disk image.
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve } from "node:path";
import { DMG_LAYOUT, dmgBackgroundSvg } from "./macos-dmg";

const root = resolve(import.meta.dir, "..");
const args = process.argv.slice(2);
const svgIndex = args.indexOf("--svg");
const svgOut = svgIndex >= 0 ? resolve(root, args[svgIndex + 1]) : undefined;

const tools = join(root, "target", "design-tools", "package.json");
if (!existsSync(tools)) throw new Error("Install resvg first: bun add --cwd target/design-tools @resvg/resvg-js@2.6.2");
const { Resvg } = createRequire(tools)("@resvg/resvg-js") as {
  Resvg: new (svg: string, options: object) => { render(): { asPng(): Buffer } };
};

const font = "/System/Library/Fonts/HelveticaNeue.ttc";
if (!existsSync(font)) throw new Error(`Helvetica Neue was not found at ${font}; render on macOS`);

const out = join(root, "assets", "macos");
mkdirSync(out, { recursive: true });
for (const scale of [1, 2]) {
  const svg = dmgBackgroundSvg(DMG_LAYOUT, scale);
  if (svgOut && scale === 1) writeFileSync(svgOut, svg);
  const png = new Resvg(svg, {
    font: { fontFiles: [font], loadSystemFonts: false, defaultFontFamily: "Helvetica Neue" },
    shapeRendering: 2,
    textRendering: 2,
  }).render().asPng();
  const name = scale === 1 ? "dmg-background.png" : "dmg-background@2x.png";
  writeFileSync(join(out, name), png);
  console.log(`Wrote assets/macos/${name} (${DMG_LAYOUT.width * scale} x ${DMG_LAYOUT.height * scale})`);
}
