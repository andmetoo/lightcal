/**
 * Layout of the Lightcal disk image window. The background art (scripts/render-dmg-background.ts)
 * and the Finder view (finderLayoutScript) both read these numbers, so the arrow and the
 * label plates always line up with the icons Finder draws.
 */
export const DMG_LAYOUT = {
  volumeName: "Lightcal",
  /** Content area of the Finder window, in points; the background is drawn at this size. */
  width: 640,
  height: 400,
  iconSize: 128,
  textSize: 13,
  /** Icon centres, as Finder positions items. */
  app: { name: "Lightcal.app", x: 170, y: 200 },
  applications: { name: "Applications", x: 470, y: 200 },
  /** Folder and file names inside the image. */
  background: ".background/background.tiff",
} as const;

export type DmgLayout = typeof DMG_LAYOUT;

/** Finder's title bar sits above the content area when the toolbar is hidden. */
const TITLE_BAR = 28;

const appleScriptString = (value: string) => `"${value.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`;

/**
 * AppleScript that sets the icon view, background, window size and icon positions. Finder
 * stores them in the volume's .DS_Store, which travels inside the compressed image.
 */
export function finderLayoutScript(layout: DmgLayout = DMG_LAYOUT): string {
  const left = 200;
  const top = 120;
  return [
    'tell application "Finder"',
    `  tell disk ${appleScriptString(layout.volumeName)}`,
    "    open",
    "    set current view of container window to icon view",
    "    set toolbar visible of container window to false",
    "    set statusbar visible of container window to false",
    "    set pathbar visible of container window to false",
    `    set the bounds of container window to {${left}, ${top}, ${left + layout.width}, ${top + layout.height + TITLE_BAR}}`,
    "    set viewOptions to the icon view options of container window",
    "    set arrangement of viewOptions to not arranged",
    `    set icon size of viewOptions to ${layout.iconSize}`,
    `    set text size of viewOptions to ${layout.textSize}`,
    // An HFS path relative to the disk; "file … of folder …" is misread as setting the folder.
    `    set background picture of viewOptions to file ${appleScriptString(layout.background.replaceAll("/", ":"))}`,
    `    set position of item ${appleScriptString(layout.app.name)} of container window to {${layout.app.x}, ${layout.app.y}}`,
    `    set position of item ${appleScriptString(layout.applications.name)} of container window to {${layout.applications.x}, ${layout.applications.y}}`,
    "    close",
    "    open",
    "    update without registering applications",
    "    delay 2",
    "    close",
    "  end tell",
    "end tell",
  ].join("\n");
}

const xml = (value: string) => value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");

/**
 * Background art for the disk image window, at 1x points (`scale` 2 gives the Retina copy).
 * Neutral Gray surfaces and a Teal action arrow follow design/palettes; coral is kept for the
 * Lightcal identity. Finder draws the icon names in dark text on this light background in
 * both Light and Dark Mode, so the art leaves the label row empty. The footer stays above
 * the 28 pt where a path bar would sit.
 */
export function dmgBackgroundSvg(layout: DmgLayout = DMG_LAYOUT, scale = 1, font = "Helvetica Neue"): string {
  const { width: w, height: h, iconSize } = layout;
  const { app, applications } = layout;
  // Arrow between the icons: a dotted Teal arc and a solid head.
  const from = app.x + iconSize / 2 + 22;
  const to = applications.x - iconSize / 2 - 22;
  const mid = (from + to) / 2;
  const y = app.y;
  const lift = 30;
  // Arrowhead along the curve's end tangent (quadratic: 2 * (end - control)).
  const tx = to - mid;
  const ty = lift;
  const length = Math.hypot(tx, ty);
  const [ux, uy] = [tx / length, ty / length];
  const head = (back: number, side: number) =>
    `${(to - ux * back - uy * side).toFixed(1)} ${(y - ux * 0 + uy * 0 - uy * back + ux * side).toFixed(1)}`;
  // Faint calendar grid behind the header, with one coral "today" cell.
  const cells: string[] = [];
  const cell = 10;
  const gap = 6;
  const gridX = w - 24 - 7 * cell - 6 * gap;
  for (let row = 0; row < 5; row++) {
    for (let col = 0; col < 7; col++) {
      const today = row === 2 && col === 4;
      cells.push(`<rect x="${gridX + col * (cell + gap)}" y="${22 + row * (cell + gap)}" width="${cell}" height="${cell}" rx="3" fill="${today ? "#D73840" : "#000000"}" fill-opacity="${today ? 0.85 : 0.045}"/>`);
    }
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${w * scale}" height="${h * scale}" viewBox="0 0 ${w} ${h}">
  <defs>
    <linearGradient id="surface" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#FBFBFB"/>
      <stop offset="1" stop-color="#ECECEC"/>
    </linearGradient>
  </defs>
  <rect width="${w}" height="${h}" fill="url(#surface)"/>
  ${cells.join("\n  ")}
  <text x="32" y="56" font-family="${xml(font)}" font-size="22" font-weight="600" fill="#202020">Install Lightcal</text>
  <text x="32" y="80" font-family="${xml(font)}" font-size="13" fill="#646464">Drag Lightcal into the Applications folder.</text>
  <path d="M ${from} ${y} Q ${mid} ${y - lift * 2} ${to - ux * 6} ${y - uy * 6}" fill="none" stroke="#00796B" stroke-width="3" stroke-linecap="round" stroke-dasharray="0.1 9"/>
  <path d="M ${head(14, -8)} L ${to} ${y} L ${head(14, 8)}" fill="none" stroke="#00796B" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
  <text x="${w / 2}" y="${h - 36}" text-anchor="middle" font-family="${xml(font)}" font-size="11" fill="#8A8A8A">Read-only calendars · No telemetry · Free and open source</text>
</svg>
`;
}
