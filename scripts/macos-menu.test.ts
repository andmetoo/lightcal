import { expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// Build menu objects and exercise a fixture delegate only. No real app is
// activated and no user preferences, credentials, or calendar data are touched.
(process.platform === "darwin" ? test : test.skip)("macOS menu states and Dock reopen preserve the toolkit delegate", async () => {
  const directory=await mkdtemp(join(tmpdir(),"lightcal-menu-fixture-"));
  try {
    const source=join(directory,"menu.m");
    await writeFile(source, `#import ${JSON.stringify(resolve(import.meta.dir,"../crates/lightcal/src/platform/macos/native.m"))}
#include <assert.h>
@interface FixtureDelegate : NSObject
- (int)lifecycleSentinel;
@end
@implementation FixtureDelegate
- (int)lifecycleSentinel { return 42; }
@end
static int received, calls;
static void record(int event) {received=event;calls++;}
int main(void) { @autoreleasepool {
    id target=[LightcalActions new];
    NSArray *titles=@[@"Open Lightcal",@"Settings…",@"Calendars…",@"Open Calendar",@"Off",@"Short",@"Full",@"Show month",@"Check for Updates…",@"Quit Lightcal",@"Weekday"];
    for(NSInteger selected=0;selected<3;selected++) {
        NSMenu *menu=buildStatusMenu(titles,selected,YES,YES,target);
        assert(menu.numberOfItems==12 && !menu.autoenablesItems);
        assert([menu itemAtIndex:3].separatorItem);
        NSMenu *choices=[menu itemAtIndex:4].submenu;
        assert(choices.numberOfItems==3);
        for(NSInteger i=0;i<3;i++) {
            NSMenuItem *choice=[choices itemAtIndex:i];
            assert(choice.tag==10+i && choice.target==target);
            assert(choice.state==(i==selected?NSControlStateValueOn:NSControlStateValueOff));
        }
        assert([menu itemWithTag:13].state==NSControlStateValueOn);
        assert([menu itemWithTag:14]!=nil && [menu itemWithTag:3]==nil);
        assert([[menu itemWithTag:2].keyEquivalent isEqualToString:@","]);
        assert([[menu itemWithTag:6].keyEquivalent isEqualToString:@"q"]);
        assert([menu itemWithTag:5].enabled);
    }
    NSMenu *off=buildStatusMenu(titles,1,NO,NO,target);
    assert([off itemWithTag:13].state==NSControlStateValueOff);
    assert(![off itemWithTag:5].enabled);
    assert(buildStatusMenu(@[],1,YES,YES,target).numberOfItems==0);
    NSMenu *main=[NSMenu new], *application=[NSMenu new];
    NSMenuItem *root=[NSMenuItem new]; root.submenu=application; [main addItem:root];
    NSMenuItem *about=commandItem(@"About",0,nil,@""); [application addItem:about];
    NSMenuItem *quit=commandItem(@"Quit",0,nil,@"q"); [application addItem:quit];
    syncApplicationCommands(main,titles,target); syncApplicationCommands(main,titles,target);
    assert(application.numberOfItems==4 && [application itemAtIndex:0]==about && [application itemAtIndex:3]==quit);
    NSMenuItem *settings=[application itemAtIndex:1], *calendars=[application itemAtIndex:2];
    assert(settings.tag==2 && settings.enabled && settings.target==target);
    assert([settings.keyEquivalent isEqualToString:@","] && settings.keyEquivalentModifierMask==NSEventModifierFlagCommand);
    assert(calendars.tag==14 && calendars.target==target);
    NSMutableArray *translated=[titles mutableCopy]; translated[1]=@"Einstellungen…"; translated[2]=@"Kalender…";
    syncApplicationCommands(main,translated,target);
    assert([settings.title isEqualToString:@"Einstellungen…"] && [calendars.title isEqualToString:@"Kalender…"]);
    assert(application.numberOfItems==4);
    FixtureDelegate *delegate=[FixtureDelegate new]; Class original=object_getClass(delegate);
    notify=record;
    installDockReopenHandler(delegate); installDockReopenHandler(delegate);
    assert(object_getClass(delegate)==original && [delegate lifecycleSentinel]==42);
    SEL reopen=@selector(applicationShouldHandleReopen:hasVisibleWindows:);
    assert([delegate respondsToSelector:reopen]);
    BOOL normal=((BOOL (*)(id,SEL,id,BOOL))objc_msgSend)(delegate,reopen,nil,NO);
    assert(!normal && received==15 && calls==1);
    normal=((BOOL (*)(id,SEL,id,BOOL))objc_msgSend)(delegate,reopen,nil,YES);
    assert(!normal && received==15 && calls==2);
    return 0;
} }
`);
    const binary=join(directory,"menu");
    const compiler=Bun.spawn(["xcrun","clang","-fobjc-arc","-fblocks","-mmacosx-version-min=14.0",source,"-o",binary,
      ...["AppKit","QuartzCore","EventKit","ServiceManagement","Carbon","Security"].flatMap(name=>["-framework",name])],{stdout:"pipe",stderr:"pipe"});
    expect(await compiler.exited,await new Response(compiler.stderr).text()).toBe(0);
    const child=Bun.spawn([binary],{stdout:"pipe",stderr:"pipe"});
    expect(await child.exited,await new Response(child.stderr).text()).toBe(0);
  } finally {await rm(directory,{recursive:true,force:true});}
});
