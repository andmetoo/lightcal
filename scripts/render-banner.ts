// Render the README banner (also the GitHub social preview) to assets/readme/banner.png.
//
// The banner is an SVG template filled with real headless app renders, drawn by resvg.
// resvg is a development-only tool and is never committed:
//
//   bun add --cwd target/design-tools @resvg/resvg-js@2.6.2
//   $env:LIGHTCAL_DATA_DIR = "$PWD\target\readme-data"
//   cargo run -p lightcal --locked -- --render-snapshots target/readme-shots
//   bun scripts/render-banner.ts [snapshot-dir] [--scale 2] [--svg target/banner.svg]
//
// Windows only: the text uses Segoe UI Variable, and static Segoe UI Semibold for the title,
// read from %WINDIR%\Fonts at render time. Fonts are never copied into the repository.
// The snapshots are 1x, so --scale 2 upscales them; the committed banner is 1x.
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const args = process.argv.slice(2);
const option = (name: string) => {
  const index = args.indexOf(name);
  if (index < 0) return undefined;
  const value = args[index + 1];
  args.splice(index, 2);
  return value;
};
const scale = Number(option("--scale") ?? "1");
const svgOut = option("--svg");
const shots = resolve(root, args[0] ?? "target/readme-shots");
const output = join(root, "assets", "readme", scale === 1 ? "banner.png" : `banner@${scale}x.png`);

const tools = join(root, "target", "design-tools", "package.json");
if (!existsSync(tools)) {
  throw new Error("Install resvg first: bun add --cwd target/design-tools @resvg/resvg-js@2.6.2");
}
const { Resvg } = createRequire(tools)("@resvg/resvg-js") as {
  Resvg: new (svg: string, options: object) => { render(): { asPng(): Buffer } };
};

const fontDir = join(process.env.WINDIR ?? "C:\\Windows", "Fonts");
const fontFiles = ["SegUIVar.ttf", "segoeui.ttf", "seguisb.ttf", "segoeuib.ttf"]
  .map((name) => join(fontDir, name))
  .filter(existsSync);
if (fontFiles.length === 0) throw new Error(`Segoe UI was not found in ${fontDir}`);

function shot(name: string): string {
  const path = join(shots, name);
  if (!existsSync(path)) throw new Error(`Missing ${path}; run --render-snapshots first`);
  return `data:image/png;base64,${readFileSync(path).toString("base64")}`;
}

const W = 1280;
const H = 640;
// Calendar renders are 352 x 432 logical pixels at 1x.
const CW = 352;
const CH = 432;
const dark = { x: 864, y: 66 };
const light = { x: 676, y: 142 };

// The app icon master (assets/icons/canvas/lightcal-app.svg), 64-unit grid.
const icon = `
  <rect x="4" y="4" width="56" height="56" rx="14" fill="#D73840"/>
  <path d="M4 18A14 14 0 0 1 18 4H46A14 14 0 0 1 60 18Z" fill="#A92A31"/>
  <rect x="18" y="25" width="12" height="12" rx="3" fill="#FFFFFF" fill-opacity=".55"/>
  <rect x="34" y="25" width="12" height="12" rx="3" fill="#FFFFFF" fill-opacity=".55"/>
  <rect x="18" y="41" width="12" height="12" rx="3" fill="#FFFFFF" fill-opacity=".55"/>
  <rect x="34" y="41" width="12" height="12" rx="3" fill="#FFFFFF"/>`;

function windowShot(href: string, x: number, y: number, shadow: string, edge: string): string {
  return `
  <g filter="url(#${shadow})">
    <rect x="${x}" y="${y}" width="${CW}" height="${CH}" rx="8" fill="#000" fill-opacity=".01"/>
    <image x="${x}" y="${y}" width="${CW}" height="${CH}" href="${href}" clip-path="url(#clip-${shadow})"/>
  </g>
  <rect x="${x + 0.5}" y="${y + 0.5}" width="${CW - 1}" height="${CH - 1}" rx="7.5" fill="none" stroke="${edge}"/>
  <clipPath id="clip-${shadow}"><rect x="${x}" y="${y}" width="${CW}" height="${CH}" rx="8"/></clipPath>`;
}

const text = "'Segoe UI Variable Text', 'Segoe UI Variable', 'Segoe UI'";

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#F7F8F8"/>
      <stop offset=".55" stop-color="#EEF3F2"/>
      <stop offset="1" stop-color="#DDEDE9"/>
    </linearGradient>
    <radialGradient id="teal" cx="0.82" cy="0.18" r="0.62">
      <stop offset="0" stop-color="#53B9AB" stop-opacity=".22"/>
      <stop offset="1" stop-color="#53B9AB" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="coral" cx="0.6" cy="0.92" r="0.42">
      <stop offset="0" stop-color="#D73840" stop-opacity=".13"/>
      <stop offset="1" stop-color="#D73840" stop-opacity="0"/>
    </radialGradient>
    <pattern id="dots" width="24" height="24" patternUnits="userSpaceOnUse">
      <circle cx="2" cy="2" r="1.1" fill="#202020" fill-opacity=".07"/>
    </pattern>
    <linearGradient id="fade" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#fff" stop-opacity="1"/>
      <stop offset=".55" stop-color="#fff" stop-opacity="0"/>
    </linearGradient>
    <mask id="dotmask"><rect width="${W}" height="${H}" fill="url(#fade)"/></mask>
    <filter id="back" x="-30%" y="-30%" width="160%" height="170%">
      <feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#0B1F1C" flood-opacity=".18"/>
      <feDropShadow dx="0" dy="24" stdDeviation="28" flood-color="#0B1F1C" flood-opacity=".32"/>
    </filter>
    <filter id="front" x="-30%" y="-30%" width="160%" height="170%">
      <feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#0B1F1C" flood-opacity=".12"/>
      <feDropShadow dx="0" dy="28" stdDeviation="30" flood-color="#0B1F1C" flood-opacity=".28"/>
    </filter>
    <filter id="iconshadow" x="-40%" y="-40%" width="180%" height="190%">
      <feDropShadow dx="0" dy="6" stdDeviation="8" flood-color="#A92A31" flood-opacity=".28"/>
    </filter>
  </defs>

  <rect width="${W}" height="${H}" fill="url(#bg)"/>
  <rect width="${W}" height="${H}" fill="url(#dots)" mask="url(#dotmask)"/>
  <rect width="${W}" height="${H}" fill="url(#teal)"/>
  <rect width="${W}" height="${H}" fill="url(#coral)"/>

  <g transform="translate(88 176) scale(1.5)" filter="url(#iconshadow)">${icon}</g>

  <text x="94" y="352" font-family="'Segoe UI Semibold', 'Segoe UI'" font-size="80" font-weight="600" letter-spacing="-1.6" fill="#1A1A1A">Lightcal</text>
  <text x="96" y="404" font-family="${text}" font-size="27" fill="#4A4F4E">A small calendar that lives</text>
  <text x="96" y="440" font-family="${text}" font-size="27" fill="#4A4F4E">in the Windows tray.</text>

  <rect x="96" y="482" width="3" height="24" rx="1.5" fill="#D73840"/>
  <text x="112" y="500" font-family="${text}" font-size="17" fill="#646464">Windows · Free and open source · GPL-3.0</text>

  ${windowShot(shot("calendar-koi-dark.png"), dark.x, dark.y, "back", "#FFFFFF14")}
  ${windowShot(shot("calendar-today-light.png"), light.x, light.y, "front", "#00000010")}
</svg>`;

if (svgOut) writeFileSync(resolve(root, svgOut), svg);

const png = new Resvg(svg, {
  fitTo: { mode: "zoom", value: scale },
  font: { fontFiles, loadSystemFonts: false, defaultFontFamily: "Segoe UI" },
  shapeRendering: 2,
  textRendering: 2,
  imageRendering: 0,
})
  .render()
  .asPng();
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, png);
console.log(`Wrote ${output} (${W * scale} x ${H * scale}, ${(png.length / 1024).toFixed(0)} KiB)`);
