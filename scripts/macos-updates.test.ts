import { expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// A stand-in for Sparkle: no framework loading, network requests or app defaults.
(process.platform === "darwin" ? test : test.skip)("UI refreshes do not reset unchanged Sparkle preferences", async () => {
  const directory = await mkdtemp(join(tmpdir(), "lightcal-updates-"));
  try {
    const source = join(directory, "updates.m");
    await writeFile(source, `#import ${JSON.stringify(resolve(import.meta.dir, "../crates/lightcal/src/platform/macos/native.m"))}
#include <assert.h>
static int checkWrites, downloadWrites;
@interface FixtureUpdater : NSObject
@property BOOL automaticallyChecksForUpdates;
@property BOOL automaticallyDownloadsUpdates;
@end
@implementation FixtureUpdater
@synthesize automaticallyChecksForUpdates=_checks;
@synthesize automaticallyDownloadsUpdates=_downloads;
- (void)setAutomaticallyChecksForUpdates:(BOOL)value { _checks=value; ++checkWrites; }
- (void)setAutomaticallyDownloadsUpdates:(BOOL)value { _downloads=value; ++downloadWrites; }
@end
int main(void) { @autoreleasepool {
    FixtureUpdater *engine=[FixtureUpdater new];
    syncUpdaterPreferences(engine,NO,NO);
    assert(checkWrites==0 && downloadWrites==0);
    syncUpdaterPreferences(engine,YES,NO);
    for(int i=0;i<100;i++) syncUpdaterPreferences(engine,YES,NO);
    assert(checkWrites==1 && downloadWrites==0);
    syncUpdaterPreferences(engine,YES,YES);
    syncUpdaterPreferences(engine,YES,YES);
    assert(checkWrites==1 && downloadWrites==1);
    syncUpdaterPreferences(engine,NO,NO);
    assert(checkWrites==2 && downloadWrites==2);
    return 0;
} }
`);
    const binary = join(directory, "updates");
    const compiler = Bun.spawn(["xcrun", "clang", "-fobjc-arc", "-fblocks", "-mmacosx-version-min=14.0", source, "-o", binary,
      ...["AppKit", "QuartzCore", "EventKit", "ServiceManagement", "Carbon", "Security"].flatMap(name => ["-framework", name])], { stdout: "pipe", stderr: "pipe" });
    const errors = await new Response(compiler.stderr).text();
    expect(await compiler.exited, errors).toBe(0);
    const child = Bun.spawn([binary], { stdout: "pipe", stderr: "pipe" });
    expect(await child.exited, await new Response(child.stderr).text()).toBe(0);
  } finally { await rm(directory, { recursive: true, force: true }); }
});
