import { expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// Test the production native geometry functions without opening an app, reading
// calendars, or changing login items or the user's application profile.
(process.platform === "darwin" ? test : test.skip)("macOS rejects placeholder menu anchors and clamps real display coordinates", async () => {
  const directory = await mkdtemp(join(tmpdir(), "lightcal-placement-"));
  try {
    const source = join(directory, "placement.m");
    await writeFile(source, `#import ${JSON.stringify(resolve(import.meta.dir, "../crates/lightcal/src/platform/macos/native.m"))}
#include <assert.h>
int main(void) {
    NSRect screen=NSMakeRect(0,0,1512,982), area=NSMakeRect(0,58,1512,891);
    NSRect frame=NSMakeRect(580,402,352,432);
    NSRect placeholder=NSMakeRect(0,-11,89,22);
    assert(!menuAnchorValid(placeholder,screen,33));
    NSRect fallback=anchoredFrame(frame,area,placeholder,NO);
    assert(fallback.origin.x==1152 && fallback.origin.y==513);
    NSRect icon=NSMakeRect(1200,960,89,22);
    assert(menuAnchorValid(icon,screen,33));
    NSRect placed=anchoredFrame(frame,area,icon,YES);
    assert(placed.origin.x==1069 && placed.origin.y==513);
    NSRect left=NSMakeRect(-1920,-1080,1920,1080), leftArea=NSMakeRect(-1920,-1080,1920,1056);
    NSRect leftIcon=NSMakeRect(-100,-24,80,22);
    assert(menuAnchorValid(leftIcon,left,24));
    NSRect other=anchoredFrame(frame,leftArea,leftIcon,YES);
    assert(other.origin.x==-360 && other.origin.y==-460);
    NSRect edge=anchoredFrame(frame,area,NSMakeRect(1510,960,2,22),YES);
    assert(edge.origin.x==1152);
    NSRect settings=anchoredFrame(NSMakeRect(0,0,448,360),area,icon,YES);
    assert(NSMaxY(settings)==NSMaxY(placed) && NSMidX(settings)==NSMidX(placed));
    // Verify monochrome menu bar icon is a true macOS template image and Dock icon matches 512pt grid.
    NSImage *monoIcon=menuBarDateImage(15,YES);
    NSImage *colorIcon=menuBarDateImage(15,NO);
    NSImage *dockIcon=dockDateImage(15);
    assert(monoIcon.isTemplate && !colorIcon.isTemplate);
    assert(monoIcon.size.width==18 && monoIcon.size.height==18);
    assert(dockIcon.size.width==512 && dockIcon.size.height==512);
    return 0;
}
`);
    const executable = join(directory, "placement");
    const compiler = Bun.spawn(["xcrun", "clang", "-fobjc-arc", "-fblocks", "-mmacosx-version-min=14.0", source, "-o", executable,
      ...["AppKit", "CoreText", "QuartzCore", "EventKit", "ServiceManagement", "Carbon", "Security"].flatMap(name => ["-framework", name])], { stdout: "pipe", stderr: "pipe" });
    const error = await new Response(compiler.stderr).text();
    expect(await compiler.exited, error).toBe(0);
    const run = Bun.spawn([executable], { stdout: "pipe", stderr: "pipe" });
    expect(await run.exited, await new Response(run.stderr).text()).toBe(0);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
