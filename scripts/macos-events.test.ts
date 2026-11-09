import { expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// Pure date/zone fixtures. Never instantiate EventKit stores or access calendars.
(process.platform === "darwin" ? test : test.skip)("EventKit civil boundaries preserve UTC, floating and DST all-day dates", async () => {
  const directory = await mkdtemp(join(tmpdir(), "lightcal-event-dates-"));
  try {
    const source = join(directory, "dates.m");
    await writeFile(source, `#import ${JSON.stringify(resolve(import.meta.dir, "../crates/lightcal/src/platform/macos/native.m"))}
#include <assert.h>
static NSDate *instant(NSString *text) { return [[NSISO8601DateFormatter new] dateFromString:text]; }
static void check(NSString *start,NSString *end,NSTimeZone *eventZone,NSTimeZone *displayZone,NSString *first,NSString *last) {
    NSDictionary *dates=allDayDates(instant(start),instant(end),eventZone,displayZone);
    assert([dates[@"start_date"] isEqualToString:first]);
    assert([dates[@"end_date_exclusive"] isEqualToString:last]);
}
int main(void) { @autoreleasepool {
    NSTimeZone *utc=[NSTimeZone timeZoneForSecondsFromGMT:0];
    NSTimeZone *la=[NSTimeZone timeZoneWithName:@"America/Los_Angeles"];
    NSTimeZone *ny=[NSTimeZone timeZoneWithName:@"America/New_York"];
    NSTimeZone *tokyo=[NSTimeZone timeZoneWithName:@"Asia/Tokyo"];
    NSTimeZone *kiritimati=[NSTimeZone timeZoneWithName:@"Pacific/Kiritimati"];
    NSTimeZone *honolulu=[NSTimeZone timeZoneWithName:@"Pacific/Honolulu"];
    // Both zones see midnight, but their civil dates differ by a full day.
    check(@"2026-11-08T10:00:00Z",@"2026-11-09T10:00:00Z",kiritimati,honolulu,@"2026-11-09",@"2026-11-10");
    check(@"2026-11-08T10:00:00Z",@"2026-11-09T10:00:00Z",honolulu,kiritimati,@"2026-11-08",@"2026-11-09");
    check(@"2026-11-09T00:00:00Z",@"2026-11-10T00:00:00Z",utc,la,@"2026-11-09",@"2026-11-10");
    check(@"2026-11-09T00:00:00Z",@"2026-11-11T00:00:00Z",nil,la,@"2026-11-09",@"2026-11-11");
    check(@"2026-11-08T15:00:00Z",@"2026-11-09T15:00:00Z",tokyo,la,@"2026-11-09",@"2026-11-10");
    // EventKit may already return floating dates in the default zone, even if
    // the source has a timezone. Do not shift them back into that source zone.
    check(@"2026-11-08T15:00:00Z",@"2026-11-09T15:00:00Z",utc,tokyo,@"2026-11-09",@"2026-11-10");
    check(@"2026-03-07T05:00:00Z",@"2026-03-09T04:00:00Z",nil,ny,@"2026-03-07",@"2026-03-09");
    check(@"2026-11-01T07:00:00Z",@"2026-11-03T08:00:00Z",nil,la,@"2026-11-01",@"2026-11-03");
    assert(!allDayDates(instant(@"2026-11-09T00:00:00Z"),instant(@"2026-11-09T00:00:00Z"),nil,la));
    return 0;
} }
`);
    const binary = join(directory, "dates");
    const compiler = Bun.spawn(["xcrun", "clang", "-fobjc-arc", "-fblocks", "-mmacosx-version-min=14.0", source, "-o", binary,
      ...["AppKit", "QuartzCore", "EventKit", "ServiceManagement", "Carbon", "Security"].flatMap(name => ["-framework", name])], { stdout: "pipe", stderr: "pipe" });
    const errors = await new Response(compiler.stderr).text();
    expect(await compiler.exited, errors).toBe(0);
    const child = Bun.spawn([binary], { stdout: "pipe", stderr: "pipe" });
    expect(await child.exited, await new Response(child.stderr).text()).toBe(0);
  } finally { await rm(directory, { recursive: true, force: true }); }
});
