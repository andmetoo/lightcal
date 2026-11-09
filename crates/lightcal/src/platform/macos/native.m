// Native services only; Slint owns the application event loop and calendar UI.
#import <AppKit/AppKit.h>
#import <Security/Security.h>
#import <QuartzCore/QuartzCore.h>
#import <EventKit/EventKit.h>
#import <ServiceManagement/ServiceManagement.h>
#import <Carbon/Carbon.h>
#import <objc/message.h>
#import <objc/runtime.h>
static void (*notify)(int);
static NSStatusItem *status;
static EKEventStore *store;
static id updater;
static EventHotKeyRef hotkey;
static BOOL settingsOpen;
static NSArray *menuTitles;
static NSInteger menuWeekday=1;
static BOOL menuShowMonth=YES;
@class LightcalActions;
static LightcalActions *actions;
static __weak NSWindow *attachedWindow;
static BOOL placementQueued;
static NSPanel *transitionSnapshot;
static NSUInteger transitionVersion;
static NSTimeInterval transitionDuration;
static BOOL transitionScheduled;
static NSRect rememberedAnchor;
static NSNumber *rememberedDisplay;
#if LIGHTCAL_NATIVE_SMOKE
static NSUInteger smokeReopens;
#endif
// A status item must be in the display's top menu strip. AppKit can supply
// a nonempty off-screen placeholder (for example {{0,-11},{89,22}}).
static BOOL menuAnchorValid(NSRect item, NSRect display, CGFloat menuHeight) {
    return isfinite(NSMinX(item)) && isfinite(NSMinY(item)) && NSWidth(item)>0 && NSHeight(item)>0
        && NSMidX(item)>=NSMinX(display) && NSMidX(item)<=NSMaxX(display)
        && NSMinY(item)>=NSMaxY(display)-menuHeight-4 && NSMaxY(item)<=NSMaxY(display)+4;
}
static NSRect anchoredFrame(NSRect frame, NSRect area, NSRect anchor, BOOL valid) {
    CGFloat right=valid ? NSMaxX(anchor) : NSMaxX(area)-8;
    CGFloat top=valid ? MIN(NSMaxY(area)-4,NSMinY(anchor)-4) : NSMaxY(area)-4;
    frame.origin.x=MAX(NSMinX(area)+8,MIN(right-NSWidth(frame),MAX(NSMinX(area)+8,NSMaxX(area)-NSWidth(frame)-8)));
    frame.origin.y=MAX(NSMinY(area)+8,top-NSHeight(frame));
    return frame;
}
static void rememberMenuClick(void) {
    NSPoint point=NSEvent.mouseLocation;
    for(NSScreen *screen in NSScreen.screens) {
        CGFloat menuHeight=MAX(NSStatusBar.systemStatusBar.thickness,NSMaxY(screen.frame)-NSMaxY(screen.visibleFrame));
        NSRect anchor=NSMakeRect(point.x,NSMaxY(screen.frame)-menuHeight,1,menuHeight);
        if(NSPointInRect(point,screen.frame) && point.y>=NSMaxY(screen.frame)-menuHeight-4 && menuAnchorValid(anchor,screen.frame,menuHeight)) {
            rememberedAnchor=anchor; rememberedDisplay=screen.deviceDescription[@"NSScreenNumber"]; return;
        }
    }
}
static void placeWindow(void);
static void schedulePlacement(void);
static void syncApplicationCommands(NSMenu *mainMenu, NSArray *titles, id target);
void lc_cancel_transition(void) {
    ++transitionVersion; transitionScheduled=NO;
    [transitionSnapshot close]; transitionSnapshot=nil;
    attachedWindow.alphaValue=1;
}
void lc_attach(void *view) { attachedWindow=((__bridge NSView *)view).window; }

void lc_labels(const char *json) {
    NSData *data=[[NSString stringWithUTF8String:json] dataUsingEncoding:NSUTF8StringEncoding];
    NSDictionary *payload=[NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
    menuTitles=payload[@"titles"];
    menuWeekday=[payload[@"weekday"] integerValue];
    menuShowMonth=[payload[@"show_month"] boolValue];
    syncApplicationCommands(NSApp.mainMenu,menuTitles,actions);
}
static NSMenuItem *commandItem(NSString *title, NSInteger code, id target, NSString *key) {
    NSMenuItem *item=[[NSMenuItem alloc] initWithTitle:title action:@selector(action:) keyEquivalent:key];
    item.target=target; item.tag=code;
    item.keyEquivalentModifierMask=NSEventModifierFlagCommand;
    return item;
}
static NSMenu *buildStatusMenu(NSArray *titles, NSInteger weekday, BOOL month, BOOL updates, id target) {
    NSMenu *menu=[NSMenu new]; menu.autoenablesItems=NO; menu.showsStateColumn=YES;
    if(titles.count!=11) return menu;
    [menu addItem:commandItem(titles[0],1,target,@"")];
    [menu addItem:commandItem(titles[1],2,target,@",")];
    [menu addItem:commandItem(titles[2],14,target,@"")];
    [menu addItem:NSMenuItem.separatorItem];
    NSMenuItem *weekdayItem=[[NSMenuItem alloc] initWithTitle:titles[10] action:nil keyEquivalent:@""];
    NSMenu *choices=[[NSMenu alloc] initWithTitle:titles[10]]; choices.autoenablesItems=NO; choices.showsStateColumn=YES;
    for(NSInteger i=0;i<3;i++) {
        NSMenuItem *item=commandItem(titles[4+i],10+i,target,@"");
        item.state=i==weekday ? NSControlStateValueOn : NSControlStateValueOff;
        [choices addItem:item];
    }
    weekdayItem.submenu=choices; [menu addItem:weekdayItem];
    NSMenuItem *monthItem=commandItem(titles[7],13,target,@"");
    monthItem.state=month ? NSControlStateValueOn : NSControlStateValueOff;
    [menu addItem:monthItem];
    [menu addItem:NSMenuItem.separatorItem];
    [menu addItem:commandItem(titles[3],4,target,@"")];
    [menu addItem:NSMenuItem.separatorItem];
    NSMenuItem *check=commandItem(titles[8],5,target,@""); check.enabled=updates;
    [menu addItem:check];
    [menu addItem:NSMenuItem.separatorItem];
    [menu addItem:commandItem(titles[9],6,target,@"q")];
    return menu;
}
// Keep the toolkit's About, Services, Hide and Quit commands. Our commands are
// also available while the calendar has keyboard focus, not just during a
// contextual menu's tracking loop. Refresh their labels when language changes.
static void syncApplicationCommands(NSMenu *mainMenu, NSArray *titles, id target) {
    if(mainMenu.numberOfItems==0 || titles.count<11 || !target) return;
    NSMenu *application=[mainMenu itemAtIndex:0].submenu;
    if(!application) return;
    NSArray *identifiers=@[@"lightcal.settings",@"lightcal.calendars"];
    for(NSUInteger index=0;index<identifiers.count;index++) {
        NSMenuItem *item=nil;
        for(NSMenuItem *candidate in application.itemArray) {
            if([candidate.identifier isEqualToString:identifiers[index]]) { item=candidate; break; }
        }
        if(!item) {
            item=commandItem(titles[index+1],index==0 ? 2 : 14,target,index==0 ? @"," : @"");
            item.identifier=identifiers[index];
            [application insertItem:item atIndex:MIN(index+1,application.numberOfItems)];
        }
        item.title=titles[index+1]; item.target=target; item.enabled=YES;
    }
}
// Extend only the optional reopen selector; retain the toolkit's delegate and
// lifecycle methods. Recheck this integration when upgrading Slint/winit.
static BOOL dockReopen(id delegate, SEL selector, NSApplication *application, BOOL visible) {
#if LIGHTCAL_NATIVE_SMOKE
    ++smokeReopens;
#endif
    if(notify) notify(15);
    return NO; // Lightcal handles restoration; suppress default document creation.
}
static void installDockReopenHandler(id delegate) {
    SEL selector=@selector(applicationShouldHandleReopen:hasVisibleWindows:);
    if(!delegate || [delegate respondsToSelector:selector]) return;
#if OBJC_BOOL_IS_BOOL
    const char *encoding="B@:@B";
#else
    const char *encoding="c@:@c";
#endif
    class_addMethod(object_getClass(delegate),selector,(IMP)dockReopen,encoding);
}
@interface LightcalActions : NSObject
@end
@implementation LightcalActions
- (void)action:(id)sender { if (notify) notify((int)[sender tag]); }
- (void)click:(id)sender {
    NSEvent *event=NSApp.currentEvent;
    if(event.type==NSEventTypeLeftMouseUp || event.type==NSEventTypeRightMouseDown || event.type==NSEventTypeRightMouseUp) rememberMenuClick();
    if (event.type == NSEventTypeRightMouseDown || event.type == NSEventTypeRightMouseUp) {
        NSMenu *menu=buildStatusMenu(menuTitles,menuWeekday,menuShowMonth,updater!=nil,self);
        [NSMenu popUpContextMenu:menu withEvent:event forView:status.button];
    } else if (notify) notify(0);
}
@end
static OSStatus shortcutHandler(EventHandlerCallRef next, EventRef event, void *data) {
    if (notify) notify(0); return noErr;
}
static NSWindow *calendarWindow(void) {
    if (attachedWindow) return attachedWindow;
    for (NSWindow *window in NSApp.windows) if ([window.title isEqualToString:@"Lightcal"]) return window;
    return nil;
}
void lc_init(void (*callback)(int)) {
    notify=callback; actions=[LightcalActions new]; store=[EKEventStore new];
    installDockReopenHandler(NSApp.delegate);
    status=[[NSStatusBar systemStatusBar] statusItemWithLength:NSVariableStatusItemLength];
    status.button.target=actions; status.button.action=@selector(click:);
    [status.button sendActionOn:NSEventMaskLeftMouseUp|NSEventMaskRightMouseDown];
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    InstallEventHandler(GetApplicationEventTarget(), shortcutHandler, 1,
        &(EventTypeSpec){kEventClassKeyboard,kEventHotKeyPressed}, NULL, NULL);
    NSNotificationCenter *center=NSNotificationCenter.defaultCenter;
    for (NSString *name in @[NSWindowDidMoveNotification, NSWindowDidResizeNotification, NSWindowDidBecomeKeyNotification]) {
        [center addObserverForName:name object:nil queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification *note) {
            if(note.object==calendarWindow()) schedulePlacement();
        }];
    }
    [center addObserverForName:NSWindowDidChangeOcclusionStateNotification object:nil queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification *note) { if(note.object==calendarWindow() && notify) notify(9); }];
    [NSWorkspace.sharedWorkspace.notificationCenter addObserverForName:NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification object:nil queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification *note) { if(NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion) lc_cancel_transition(); if(notify) notify(9); }];
    [center addObserverForName:NSApplicationDidFinishLaunchingNotification object:nil queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification *note) {
        installDockReopenHandler(NSApp.delegate);
        syncApplicationCommands(NSApp.mainMenu,menuTitles,actions);
        schedulePlacement();
        NSAppleEventDescriptor *event=NSAppleEventManager.sharedAppleEventManager.currentAppleEvent;
        if ([[event paramDescriptorForKeyword:keyAEPropData] descriptorForKeyword:keyAELaunchedAsLogInItem].booleanValue && notify) notify(8);
    }];
    for (NSString *name in @[EKEventStoreChangedNotification, NSSystemTimeZoneDidChangeNotification, NSCalendarDayChangedNotification, NSApplicationDidChangeScreenParametersNotification]) {
        [center addObserverForName:name object:nil queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification *note) { schedulePlacement(); if(notify) notify(7); }];
    }
    [NSWorkspace.sharedWorkspace.notificationCenter addObserverForName:NSWorkspaceDidWakeNotification object:nil queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification *note) { if(notify) notify(7); }];
    [center addObserverForName:NSWindowDidResignKeyNotification object:nil queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification *note) {
        if (note.object==calendarWindow() && !settingsOpen && notify) notify(8);
    }];
    [NSEvent addGlobalMonitorForEventsMatchingMask:NSEventMaskLeftMouseDown|NSEventMaskRightMouseDown handler:^(NSEvent *event) {
        if (!settingsOpen && notify) notify(8);
    }];
    NSBundle *framework=[NSBundle bundleWithPath:[NSBundle.mainBundle.privateFrameworksPath stringByAppendingPathComponent:@"Sparkle.framework"]];
    if ([framework load]) {
        Class cls=NSClassFromString(@"SPUStandardUpdaterController");
        updater=((id (*)(id,SEL,BOOL,id,id))objc_msgSend)([cls alloc], @selector(initWithStartingUpdater:updaterDelegate:userDriverDelegate:), YES, nil, nil);
    }
}
// Draw at the requested logical size; AppKit chooses the display's backing scale.
static NSImage *dateImage(CGFloat side, int day, BOOL mono) {
    return [NSImage imageWithSize:NSMakeSize(side,side) flipped:NO drawingHandler:^BOOL(NSRect rect) {
        CGFloat scale=side/20.0;
        NSColor *ink=mono ? NSColor.labelColor : NSColor.whiteColor;
        NSBezierPath *tile=[NSBezierPath bezierPathWithRoundedRect:NSInsetRect(rect,scale,scale) xRadius:3*scale yRadius:3*scale];
        if(mono) { [ink setStroke]; tile.lineWidth=scale; [tile stroke]; }
        else { [[NSColor colorWithSRGBRed:0.843 green:0.220 blue:0.251 alpha:1] setFill]; [tile fill]; }
        NSString *digits=[NSString stringWithFormat:@"%d",day];
        NSDictionary *attributes=@{NSFontAttributeName:[NSFont boldSystemFontOfSize:(day<10?13:11)*scale],NSForegroundColorAttributeName:ink};
        NSSize inkSize=[digits sizeWithAttributes:attributes];
        [digits drawAtPoint:NSMakePoint((side-inkSize.width)/2,(side-inkSize.height)/2) withAttributes:attributes];
        return YES;
    }];
}
static void syncUpdaterPreferences(id engine, BOOL automatic, BOOL download) {
    // Setting these properties can reset Sparkle's cycle. UI synchronization is
    // frequent; write only when the actual preference changes.
    if([[engine valueForKey:@"automaticallyChecksForUpdates"] boolValue]!=automatic)
        [engine setValue:@(automatic) forKey:@"automaticallyChecksForUpdates"];
    if([[engine valueForKey:@"automaticallyDownloadsUpdates"] boolValue]!=download)
        [engine setValue:@(download) forKey:@"automaticallyDownloadsUpdates"];
}
void lc_update(const char *text, int day, bool mono, bool settings, bool automatic, bool download) {
    if (!status) return;
    settingsOpen=settings;
    status.button.title=[NSString stringWithUTF8String:text];
    status.button.font=[NSFont systemFontOfSize:14 weight:NSFontWeightMedium];
    status.button.image=dateImage(20,day,mono); status.button.imagePosition=NSImageLeft;
    // The running Dock icon is independent of bundle-icon caching and always colored.
    static int dockDay=0;
    if (dockDay!=day) {
        NSApp.applicationIconImage=dateImage(512,day,NO);
        [NSApp.dockTile display];
        dockDay=day;
    }
    if(updater) {
        id engine=((id (*)(id,SEL))objc_msgSend)(updater,@selector(updater));
        syncUpdaterPreferences(engine,automatic,download);
    }
}
int lc_visible(void) { NSWindow *window=calendarWindow(); return window.visible ? ((window.occlusionState & NSWindowOcclusionStateVisible) ? 2 : 1) : 0; }
// Placement never activates or opens a window. A deferred pass runs after Slint
// applies its size constraints, including the first show and monitor changes.
static void placeWindow(void) {
    NSWindow *window=calendarWindow(); if(!window || !window.visible) return;
    window.opaque=NO; window.backgroundColor=NSColor.clearColor; window.hasShadow=YES;
    window.animationBehavior=NSWindowAnimationBehaviorNone;
    window.contentView.wantsLayer=YES;
    window.contentView.layer.cornerRadius=8.0;
    window.contentView.layer.masksToBounds=YES;
    NSWindow *bar=status.button.window;
    NSScreen *screen=bar.screen ?: window.screen ?: NSScreen.mainScreen;
    if(!screen) return;
    NSRect area=screen.visibleFrame, frame=window.frame;
    NSRect button=bar ? [bar convertRectToScreen:[status.button convertRect:status.button.bounds toView:nil]] : NSZeroRect;
    CGFloat menuHeight=MAX(NSStatusBar.systemStatusBar.thickness,NSMaxY(screen.frame)-NSMaxY(area));
    BOOL validAnchor=bar && menuAnchorValid(button,screen.frame,menuHeight);
    if(validAnchor) { rememberedAnchor=button; rememberedDisplay=screen.deviceDescription[@"NSScreenNumber"]; }
    else if(rememberedDisplay) {
        for(NSScreen *candidate in NSScreen.screens) {
            CGFloat height=MAX(NSStatusBar.systemStatusBar.thickness,NSMaxY(candidate.frame)-NSMaxY(candidate.visibleFrame));
            if([candidate.deviceDescription[@"NSScreenNumber"] isEqual:rememberedDisplay] && menuAnchorValid(rememberedAnchor,candidate.frame,height)) {
                screen=candidate; area=screen.visibleFrame; button=rememberedAnchor; validAnchor=YES; break;
            }
        }
    }
    frame=anchoredFrame(frame,area,button,validAnchor);
    // Opt-in, bounded geometry diagnostics; no calendar or account data.
    static int diagnostics=0;
    if(getenv("LIGHTCAL_DEBUG_PLACEMENT") && diagnostics++<32) {
        fprintf(stderr,"Lightcal placement: current=%s screen=%s area=%s item=%s target=%s anchor=%d\n",NSStringFromRect(window.frame).UTF8String,NSStringFromRect(screen.frame).UTF8String,NSStringFromRect(area).UTF8String,NSStringFromRect(button).UTF8String,NSStringFromRect(frame).UTF8String,validAnchor);
    }
    if(!NSEqualRects(window.frame,frame)) { [window setFrame:frame display:YES]; [window invalidateShadow]; }
}
static void schedulePlacement(void) {
    if(placementQueued) return;
    placementQueued=YES;
    dispatch_async(dispatch_get_main_queue(), ^{ placementQueued=NO; placeWindow(); });
}
void lc_place(void) { placeWindow(); schedulePlacement(); }
void lc_show(void) {
    lc_cancel_transition();
    [NSApp activateIgnoringOtherApps:YES];
    NSWindow *window=calendarWindow();
    if(window.miniaturized) [window deminiaturize:nil];
    placeWindow();
    [window makeKeyAndOrderFront:nil];
    schedulePlacement();
}
#if LIGHTCAL_NATIVE_SMOKE
// Exercise AppKit's real AppleEvent dispatch, not a direct delegate invocation.
// These entry points are absent from normal builds and target this process only.
int lc_smoke_reopen(void) {
    pid_t process=getpid();
    AEAddressDesc address={typeNull,NULL}; AppleEvent event={typeNull,NULL};
    OSStatus result=AECreateDesc(typeKernelProcessID,&process,sizeof(process),&address);
    if(result==noErr) result=AECreateAppleEvent(kCoreEventClass,kAEReopenApplication,&address,kAutoGenerateReturnID,kAnyTransactionID,&event);
    if(result==noErr) result=AESendMessage(&event,NULL,kAENoReply,kAEDefaultTimeout);
    AEDisposeDesc(&event); AEDisposeDesc(&address);
    return result;
}
bool lc_smoke_settings(void) {
    NSEvent *key=[NSEvent keyEventWithType:NSEventTypeKeyDown location:NSZeroPoint
        modifierFlags:NSEventModifierFlagCommand timestamp:NSProcessInfo.processInfo.systemUptime
        windowNumber:calendarWindow().windowNumber context:nil characters:@"," charactersIgnoringModifiers:@","
        isARepeat:NO keyCode:43];
    return [NSApp.mainMenu performKeyEquivalent:key];
}
char *lc_smoke_state(void) {
    NSWindow *window=calendarWindow();
    NSRect frame=window.frame, area=window.screen.visibleFrame;
    BOOL anchored=window && fabs(NSMaxY(frame)-(NSMaxY(area)-4))<1
        && NSMinX(frame)>=NSMinX(area)+7 && NSMaxX(frame)<=NSMaxX(area)-7;
    NSDictionary *state=@{@"visible":window.visible ? @YES : @NO,@"miniaturized":window.miniaturized ? @YES : @NO,
        @"anchored":anchored ? @YES : @NO,@"alpha":@(window.alphaValue),
        @"snapshot":transitionSnapshot ? @YES : @NO,@"reopens":@(smokeReopens),
        @"delegate":[NSApp.delegate respondsToSelector:@selector(applicationShouldHandleReopen:hasVisibleWindows:)] ? @YES : @NO};
    NSData *json=[NSJSONSerialization dataWithJSONObject:state options:0 error:nil];
    return strdup([[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding].UTF8String);
}
#endif
// Snapshot the outgoing surface once; only native window opacity animates.
// Neither Slint layout nor the software renderer runs once per animation frame.
void lc_begin_transition(double duration) {
    NSWindow *window=calendarWindow();
    if(!attachedWindow || !window.visible || NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion || duration<=0) return;
    if(transitionSnapshot) return;
    ++transitionVersion;
    transitionDuration=duration;
    transitionScheduled=NO; // Retarget rapid navigation without queuing snapshots.
    NSView *view=window.contentView;
    if(!view.layer || NSWidth(view.bounds)<=0 || NSHeight(view.bounds)<=0) return;
    CGFloat scale=window.backingScaleFactor;
    NSBitmapImageRep *bitmap=[[NSBitmapImageRep alloc] initWithBitmapDataPlanes:NULL pixelsWide:ceil(NSWidth(view.bounds)*scale) pixelsHigh:ceil(NSHeight(view.bounds)*scale) bitsPerSample:8 samplesPerPixel:4 hasAlpha:YES isPlanar:NO colorSpaceName:NSDeviceRGBColorSpace bytesPerRow:0 bitsPerPixel:0];
    if(!bitmap) return;
    bitmap.size=view.bounds.size;
    NSGraphicsContext *context=[NSGraphicsContext graphicsContextWithBitmapImageRep:bitmap];
    CGContextScaleCTM(context.CGContext,scale,scale);
    [view.layer renderInContext:context.CGContext];
    NSImage *image=[[NSImage alloc] initWithSize:view.bounds.size]; [image addRepresentation:bitmap];
    transitionSnapshot=[[NSPanel alloc] initWithContentRect:window.frame styleMask:NSWindowStyleMaskBorderless|NSWindowStyleMaskNonactivatingPanel backing:NSBackingStoreBuffered defer:NO];
    transitionSnapshot.releasedWhenClosed=NO; transitionSnapshot.ignoresMouseEvents=YES;
    transitionSnapshot.opaque=NO; transitionSnapshot.backgroundColor=NSColor.clearColor;
    transitionSnapshot.hasShadow=YES; transitionSnapshot.level=window.level;
    transitionSnapshot.animationBehavior=NSWindowAnimationBehaviorNone;
    NSImageView *copy=[[NSImageView alloc] initWithFrame:view.bounds]; copy.image=image;
    copy.imageScaling=NSImageScaleAxesIndependently; copy.wantsLayer=YES;
    copy.layer.cornerRadius=8; copy.layer.masksToBounds=YES;
    transitionSnapshot.contentView=copy;
    [transitionSnapshot orderWindow:NSWindowAbove relativeTo:window.windowNumber];
    window.alphaValue=0;
}
void lc_finish_transition(void) {
    if(!transitionSnapshot || transitionScheduled) return;
    transitionScheduled=YES;
    NSUInteger version=transitionVersion;
    dispatch_async(dispatch_get_main_queue(), ^{
        NSWindow *window=calendarWindow();
        if(version!=transitionVersion) return;
        if(!window.visible || NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion) { lc_cancel_transition(); return; }
        placeWindow();
        [NSAnimationContext runAnimationGroup:^(NSAnimationContext *context) {
            context.duration=transitionDuration;
            context.timingFunction=[CAMediaTimingFunction functionWithControlPoints:0.23 :1 :0.32 :1];
            window.animator.alphaValue=1;
            transitionSnapshot.animator.alphaValue=0;
        } completionHandler:^{ if(version==transitionVersion) lc_cancel_transition(); }];
    });
}
void lc_open(const char *url) { [NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:[NSString stringWithUTF8String:url]]]; }
void lc_calendar(void) { [NSWorkspace.sharedWorkspace openURL:[NSURL fileURLWithPath:@"/System/Applications/Calendar.app"]]; }
void lc_permission(void) {
    [store requestFullAccessToEventsWithCompletion:^(BOOL granted,NSError *error) { dispatch_async(dispatch_get_main_queue(), ^{ if(notify) notify(7); }); }];
}
bool lc_dark(void) { return [[NSUserDefaults.standardUserDefaults stringForKey:@"AppleInterfaceStyle"] isEqualToString:@"Dark"]; }
bool lc_motion(void) { return !NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion; }
int lc_startup(bool enable) {
    NSError *error=nil; SMAppService *service=SMAppService.mainAppService;
    BOOL ok=enable?[service registerAndReturnError:&error]:[service unregisterAndReturnError:&error];
    return ok && (!enable || service.status==SMAppServiceStatusEnabled) ? 0 : 1;
}
int lc_startup_status(void) { return SMAppService.mainAppService.status==SMAppServiceStatusEnabled; }
bool lc_shortcut(int code, unsigned modifiers) {
    EventHotKeyRef replacement=NULL;
    if(code>=0 && RegisterEventHotKey(code,modifiers,(EventHotKeyID){'LCAL',1},GetApplicationEventTarget(),0,&replacement)!=noErr) return false;
    if(hotkey) UnregisterEventHotKey(hotkey); hotkey=replacement; return true;
}
bool lc_updater_ready(void) { return updater!=nil; }
void lc_check_updates(void) { if(updater) ((void (*)(id,SEL,id))objc_msgSend)(updater,@selector(checkForUpdates:),nil); }
void lc_copy(const char *text) { [NSPasteboard.generalPasteboard clearContents]; [NSPasteboard.generalPasteboard setString:[NSString stringWithUTF8String:text] forType:NSPasteboardTypeString]; }
static BOOL midnightInZone(NSDate *date, NSTimeZone *zone) {
    NSCalendar *calendar=[[NSCalendar alloc] initWithCalendarIdentifier:NSCalendarIdentifierGregorian];
    calendar.timeZone=zone;
    NSDateComponents *parts=[calendar components:NSCalendarUnitHour|NSCalendarUnitMinute|NSCalendarUnitSecond fromDate:date];
    return parts.hour==0 && parts.minute==0 && parts.second==0;
}
// Export civil boundaries before crossing FFI. EventKit floating dates normally
// use the default zone; explicit event zones and UTC-midnight source dates must
// not be shifted into the Rust process's current local day.
static NSDictionary *allDayDates(NSDate *start, NSDate *end, NSTimeZone *eventZone, NSTimeZone *displayZone) {
    if(!start || !end || [end compare:start]!=NSOrderedDescending) return nil;
    NSTimeZone *utc=[NSTimeZone timeZoneForSecondsFromGMT:0];
    NSTimeZone *zone=eventZone ?: displayZone;
    if(eventZone && midnightInZone(start,eventZone) && midnightInZone(end,eventZone)) zone=eventZone;
    else if(midnightInZone(start,displayZone) && midnightInZone(end,displayZone)) zone=displayZone;
    else if(midnightInZone(start,utc) && midnightInZone(end,utc)) zone=utc;
    NSDateFormatter *formatter=[NSDateFormatter new];
    formatter.locale=[[NSLocale alloc] initWithLocaleIdentifier:@"en_US_POSIX"];
    formatter.calendar=[[NSCalendar alloc] initWithCalendarIdentifier:NSCalendarIdentifierGregorian];
    formatter.timeZone=zone; formatter.dateFormat=@"yyyy-MM-dd";
    return @{@"start_date":[formatter stringFromDate:start],@"end_date_exclusive":[formatter stringFromDate:end]};
}
// Called only on a background worker. A separate store avoids cross-thread EventKit objects.
char *lc_events(double from, double to) {
    @autoreleasepool {
        EKAuthorizationStatus authorization=[EKEventStore authorizationStatusForEntityType:EKEntityTypeEvent];
        if(authorization!=EKAuthorizationStatusFullAccess) return strdup("{\"denied\":true,\"sources\":[],\"events\":[]}");
        EKEventStore *reader=[EKEventStore new];
        NSArray *calendars=[reader calendarsForEntityType:EKEntityTypeEvent];
        NSMutableArray *sources=[NSMutableArray new], *events=[NSMutableArray new];
        for(EKCalendar *calendar in calendars) {
            NSColor *color=[NSColor colorWithCGColor:calendar.CGColor]; color=[color colorUsingColorSpace:NSColorSpace.sRGBColorSpace];
            NSString *hex=color?[NSString stringWithFormat:@"#%02X%02X%02X",(int)(color.redComponent*255),(int)(color.greenComponent*255),(int)(color.blueComponent*255)]:@"#008573";
            [sources addObject:@{@"id":calendar.calendarIdentifier,@"name":calendar.title?:@"",@"account":calendar.source.title?:@"macOS",@"color":hex}];
        }
        NSPredicate *predicate=[reader predicateForEventsWithStartDate:[NSDate dateWithTimeIntervalSince1970:from] endDate:[NSDate dateWithTimeIntervalSince1970:to] calendars:calendars];
        for(EKEvent *event in [reader eventsMatchingPredicate:predicate]) {
            if(events.count>=20000) break;
            NSMutableDictionary *row=[@{@"id":event.eventIdentifier?:@"",@"source":event.calendar.calendarIdentifier,@"title":event.title?:@"",@"location":event.location?:@"",@"all_day":event.allDay ? @YES : @NO,@"start":@(event.startDate.timeIntervalSince1970),@"end":@(event.endDate.timeIntervalSince1970)} mutableCopy];
            if(event.allDay) {
                NSDictionary *dates=allDayDates(event.startDate,event.endDate,event.timeZone,NSTimeZone.defaultTimeZone);
                if(!dates) continue;
                [row addEntriesFromDictionary:dates];
            }
            [events addObject:row];
        }
        NSData *data=[NSJSONSerialization dataWithJSONObject:@{@"denied":@NO,@"sources":sources,@"events":events} options:0 error:nil];
        return strdup([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String);
    }
}
void lc_free(char *value) { free(value); }

// OAuth credentials live in this app's generic-password item, never preferences.
// These blocking Keychain operations are called only by provider workers.
static NSDictionary *credentialQuery(void) {
    return @{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,
        (__bridge id)kSecAttrService:@"app.koona.lightcal.accounts",
        (__bridge id)kSecAttrAccount:@"default",
        (__bridge id)kSecAttrSynchronizable:@NO};
}
int lc_credentials_read(char **output) {
    @autoreleasepool {
        *output=NULL;
        NSMutableDictionary *query=[credentialQuery() mutableCopy];
        query[(__bridge id)kSecReturnData]=@YES;
        query[(__bridge id)kSecMatchLimit]=(__bridge id)kSecMatchLimitOne;
        CFTypeRef result=NULL;
        OSStatus code=SecItemCopyMatching((__bridge CFDictionaryRef)query,&result);
        if(code==errSecItemNotFound) return 0;
        if(code!=errSecSuccess) return code;
        NSData *data=CFBridgingRelease(result);
        if(data.length>1024*1024) return errSecDecode;
        char *bytes=malloc(data.length+1); if(!bytes) return errSecAllocate;
        memcpy(bytes,data.bytes,data.length); bytes[data.length]=0; *output=bytes;
        return 0;
    }
}
int lc_credentials_write(const char *json, bool remove) {
    @autoreleasepool {
        NSDictionary *query=credentialQuery();
        if(remove) {
            OSStatus code=SecItemDelete((__bridge CFDictionaryRef)query);
            return code==errSecItemNotFound ? 0 : code;
        }
        NSData *data=[[NSString stringWithUTF8String:json] dataUsingEncoding:NSUTF8StringEncoding];
        NSDictionary *values=@{(__bridge id)kSecValueData:data};
        OSStatus code=SecItemUpdate((__bridge CFDictionaryRef)query,(__bridge CFDictionaryRef)values);
        if(code==errSecItemNotFound) {
            NSMutableDictionary *item=[query mutableCopy];
            item[(__bridge id)kSecValueData]=data;
            item[(__bridge id)kSecAttrLabel]=@"Lightcal calendar accounts";
            code=SecItemAdd((__bridge CFDictionaryRef)item,NULL);
        }
        return code;
    }
}
