// Native macOS services only; Slint owns the application event loop and calendar UI.
#import <AppKit/AppKit.h>
#import <Carbon/Carbon.h>
#import <CoreText/CoreText.h>
#import <EventKit/EventKit.h>
#import <QuartzCore/QuartzCore.h>
#import <Security/Security.h>
#import <ServiceManagement/ServiceManagement.h>
#import <objc/message.h>
#import <objc/runtime.h>

enum {
    LC_EVENT_TOGGLE = 0,
    LC_EVENT_OPEN = 1,
    LC_EVENT_OPEN_SETTINGS = 2,
    LC_EVENT_REQUEST_ACCESS = 3,
    LC_EVENT_OPEN_CALENDAR_APP = 4,
    LC_EVENT_CHECK_UPDATES = 5,
    LC_EVENT_QUIT = 6,
    LC_EVENT_REFRESH = 7,
    LC_EVENT_DISMISS = 8,
    LC_EVENT_VIEW_CHANGED = 9,
    LC_EVENT_WEEKDAY_OFF = 10,
    LC_EVENT_WEEKDAY_SHORT = 11,
    LC_EVENT_WEEKDAY_FULL = 12,
    LC_EVENT_TOGGLE_MONTH = 13,
    LC_EVENT_OPEN_ACCOUNTS = 14,
    LC_EVENT_DOCK_REOPEN = 15,
    LC_EVENT_THEME_CHANGED = 16,
};

static void (*notify)(int);
static NSStatusItem *status;
static EKEventStore *permissionStore;
static id updater;
static EventHotKeyRef hotkey;
static BOOL settingsOpen;
static BOOL settingsMovedByUser;
static BOOL preferredDarkAppearance;
static BOOL hasPreferredAppearance;
static NSArray *menuTitles;
static NSInteger menuWeekday = 1;
static BOOL menuShowMonth = YES;
@class LightcalActions;
static LightcalActions *actions;
static __weak NSWindow *attachedWindow;
static __weak NSWindow *configuredWindow;
static BOOL placementQueued;
static NSPanel *transitionSnapshot;
static NSUInteger transitionVersion;
static NSTimeInterval transitionDuration;
static BOOL transitionScheduled;
static NSRect rememberedAnchor;
static NSNumber *rememberedDisplay;
static NSTimeInterval lastStatusClickTime;

#if LIGHTCAL_NATIVE_SMOKE
static NSUInteger smokeReopens;
#endif

static void placeWindow(void);
static void schedulePlacement(void);
static void syncApplicationCommands(NSMenu *mainMenu, NSArray *titles, id target);
static NSWindow *calendarWindow(void);

// A status item must lie inside the display's top menu strip. AppKit can expose
// a nonempty off-screen placeholder (for example {{0, -11}, {89, 22}}).
static BOOL menuAnchorValid(NSRect item, NSRect display, CGFloat menuHeight) {
    return isfinite(NSMinX(item)) && isfinite(NSMinY(item))
        && NSWidth(item) > 0 && NSHeight(item) > 0
        && NSMidX(item) >= NSMinX(display) && NSMidX(item) <= NSMaxX(display)
        && NSMinY(item) >= NSMaxY(display) - menuHeight - 4.0
        && NSMaxY(item) <= NSMaxY(display) + 4.0;
}

// Center the popup beneath its menu bar item and clamp cleanly inside the display's
// visible work area with an 8 pt margin. Falls back to top-right when unanchored.
static NSRect anchoredFrame(NSRect frame, NSRect area, NSRect anchor, BOOL valid) {
    const CGFloat margin = 8.0;
    CGFloat minX = NSMinX(area) + margin;
    CGFloat maxX = MAX(minX, NSMaxX(area) - NSWidth(frame) - margin);
    CGFloat desiredX = valid
        ? floor(NSMidX(anchor) - NSWidth(frame) * 0.5 + 0.5)
        : maxX;
    CGFloat top = valid
        ? MIN(NSMaxY(area) - 4.0, NSMinY(anchor) - 4.0)
        : NSMaxY(area) - 4.0;
    frame.origin.x = MAX(minX, MIN(desiredX, maxX));
    frame.origin.y = MAX(NSMinY(area) + margin, top - NSHeight(frame));
    return frame;
}

static NSRect clampFrameToArea(NSRect frame, NSRect area) {
    const CGFloat margin = 8.0;
    CGFloat minX = NSMinX(area) + margin;
    CGFloat maxX = MAX(minX, NSMaxX(area) - NSWidth(frame) - margin);
    CGFloat minY = NSMinY(area) + margin;
    CGFloat maxY = MAX(minY, NSMaxY(area) - NSHeight(frame) - 4.0);
    frame.origin.x = MAX(minX, MIN(frame.origin.x, maxX));
    frame.origin.y = MAX(minY, MIN(frame.origin.y, maxY));
    return frame;
}

static BOOL pointInsideStatusButton(NSPoint screenPoint) {
    if (!status || !status.button || !status.button.window) return NO;
    NSRect buttonRect = [status.button.window convertRectToScreen:
        [status.button convertRect:status.button.bounds toView:nil]];
    return NSPointInRect(screenPoint, NSInsetRect(buttonRect, -2.0, -2.0));
}

static void rememberMenuClick(void) {
    NSPoint point = NSEvent.mouseLocation;
    for (NSScreen *screen in NSScreen.screens) {
        CGFloat menuHeight = MAX(
            NSStatusBar.systemStatusBar.thickness,
            NSMaxY(screen.frame) - NSMaxY(screen.visibleFrame));
        NSRect anchor = NSMakeRect(point.x, NSMaxY(screen.frame) - menuHeight, 1.0, menuHeight);
        if (NSPointInRect(point, screen.frame)
            && point.y >= NSMaxY(screen.frame) - menuHeight - 4.0
            && menuAnchorValid(anchor, screen.frame, menuHeight)) {
            rememberedAnchor = anchor;
            rememberedDisplay = screen.deviceDescription[@"NSScreenNumber"];
            return;
        }
    }
}

static void applyWindowAppearance(NSWindow *window) {
    if (!window || !hasPreferredAppearance) return;
    NSAppearanceName name = preferredDarkAppearance
        ? NSAppearanceNameDarkAqua
        : NSAppearanceNameAqua;
    NSAppearance *appearance = [NSAppearance appearanceNamed:name];
    if (appearance && ![window.appearance.name isEqualToString:name]) {
        window.appearance = appearance;
    }
}

static void applyWindowMode(NSWindow *window, BOOL settings) {
    if (!window) return;
    if (settings) {
        if (window.level != NSNormalWindowLevel) {
            window.level = NSNormalWindowLevel;
        }
        NSWindowCollectionBehavior behavior = NSWindowCollectionBehaviorMoveToActiveSpace;
        if (window.collectionBehavior != behavior) {
            window.collectionBehavior = behavior;
        }
    } else {
        if (window.level != NSStatusWindowLevel) {
            window.level = NSStatusWindowLevel;
        }
        NSWindowCollectionBehavior behavior =
            NSWindowCollectionBehaviorMoveToActiveSpace
            | NSWindowCollectionBehaviorFullScreenAuxiliary
            | NSWindowCollectionBehaviorTransient;
        if (window.collectionBehavior != behavior) {
            window.collectionBehavior = behavior;
        }
    }
}

static void configureWindowChrome(NSWindow *window) {
    if (!window) return;
    if (configuredWindow != window) {
        window.opaque = NO;
        window.backgroundColor = NSColor.clearColor;
        window.hasShadow = YES;
        window.animationBehavior = NSWindowAnimationBehaviorNone;
        window.movableByWindowBackground = NO;
        if (window.contentView) {
            window.contentView.wantsLayer = YES;
            window.contentView.layer.cornerRadius = 8.0;
            if (@available(macOS 10.15, *)) {
                window.contentView.layer.cornerCurve = kCACornerCurveContinuous;
            }
            window.contentView.layer.masksToBounds = YES;
        }
        configuredWindow = window;
    }
    applyWindowAppearance(window);
    applyWindowMode(window, settingsOpen);
}

void lc_cancel_transition(void) {
    ++transitionVersion;
    transitionScheduled = NO;
    [transitionSnapshot close];
    transitionSnapshot = nil;
    attachedWindow.alphaValue = 1.0;
}

void lc_attach(void *view) {
    if (!view) return;
    NSWindow *window = ((__bridge NSView *)view).window;
    if (!window) return;
    attachedWindow = window;
    configureWindowChrome(window);
}

void lc_set_appearance(bool dark) {
    preferredDarkAppearance = dark ? YES : NO;
    hasPreferredAppearance = YES;
    NSWindow *window = calendarWindow();
    if (window) {
        applyWindowAppearance(window);
        [window invalidateShadow];
    }
}

void lc_start_drag(void) {
    if (!settingsOpen) return;
    NSWindow *window = calendarWindow();
    NSEvent *event = NSApp.currentEvent;
    if (!window || !event) return;
    if (event.type == NSEventTypeLeftMouseDown || event.type == NSEventTypeLeftMouseDragged) {
        settingsMovedByUser = YES;
        [window performWindowDragWithEvent:event];
    }
}

void lc_labels(const char *json) {
    if (!json) return;
    NSString *text = [NSString stringWithUTF8String:json];
    if (!text) return;
    NSData *data = [text dataUsingEncoding:NSUTF8StringEncoding];
    NSDictionary *payload = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
    if (![payload isKindOfClass:[NSDictionary class]]) return;
    NSArray *titles = payload[@"titles"];
    if ([titles isKindOfClass:[NSArray class]]) {
        menuTitles = titles;
    }
    menuWeekday = [payload[@"weekday"] integerValue];
    menuShowMonth = [payload[@"show_month"] boolValue];
    syncApplicationCommands(NSApp.mainMenu, menuTitles, actions);
}

static NSMenuItem *commandItem(NSString *title, NSInteger code, id target, NSString *key) {
    NSMenuItem *item = [[NSMenuItem alloc]
        initWithTitle:title ?: @""
               action:@selector(action:)
        keyEquivalent:key ?: @""];
    item.target = target;
    item.tag = code;
    item.keyEquivalentModifierMask = NSEventModifierFlagCommand;
    return item;
}

static NSMenu *buildStatusMenu(
    NSArray *titles,
    NSInteger weekday,
    BOOL month,
    BOOL updates,
    id target
) {
    NSMenu *menu = [NSMenu new];
    menu.autoenablesItems = NO;
    menu.showsStateColumn = YES;
    if (titles.count != 11) return menu;

    [menu addItem:commandItem(titles[0], LC_EVENT_OPEN, target, @"")];
    [menu addItem:commandItem(titles[1], LC_EVENT_OPEN_SETTINGS, target, @",")];
    [menu addItem:commandItem(titles[2], LC_EVENT_OPEN_ACCOUNTS, target, @"")];
    [menu addItem:NSMenuItem.separatorItem];

    NSMenuItem *weekdayItem = [[NSMenuItem alloc] initWithTitle:titles[10] action:nil keyEquivalent:@""];
    NSMenu *choices = [[NSMenu alloc] initWithTitle:titles[10]];
    choices.autoenablesItems = NO;
    choices.showsStateColumn = YES;
    for (NSInteger i = 0; i < 3; i++) {
        NSMenuItem *item = commandItem(titles[4 + i], LC_EVENT_WEEKDAY_OFF + i, target, @"");
        item.state = (i == weekday) ? NSControlStateValueOn : NSControlStateValueOff;
        [choices addItem:item];
    }
    weekdayItem.submenu = choices;
    [menu addItem:weekdayItem];

    NSMenuItem *monthItem = commandItem(titles[7], LC_EVENT_TOGGLE_MONTH, target, @"");
    monthItem.state = month ? NSControlStateValueOn : NSControlStateValueOff;
    [menu addItem:monthItem];
    [menu addItem:NSMenuItem.separatorItem];

    [menu addItem:commandItem(titles[3], LC_EVENT_OPEN_CALENDAR_APP, target, @"")];
    [menu addItem:NSMenuItem.separatorItem];

    NSMenuItem *check = commandItem(titles[8], LC_EVENT_CHECK_UPDATES, target, @"");
    check.enabled = updates;
    [menu addItem:check];
    [menu addItem:NSMenuItem.separatorItem];

    [menu addItem:commandItem(titles[9], LC_EVENT_QUIT, target, @"q")];
    return menu;
}

// Keep the toolkit's About, Services, Hide and Quit commands. Our commands are
// also available while the calendar has keyboard focus, not just during a
// contextual menu's tracking loop. Refresh their labels when language changes.
static void syncApplicationCommands(NSMenu *mainMenu, NSArray *titles, id target) {
    if (mainMenu.numberOfItems == 0 || titles.count < 11 || !target) return;
    NSMenu *application = [mainMenu itemAtIndex:0].submenu;
    if (!application) return;
    NSArray *identifiers = @[@"lightcal.settings", @"lightcal.calendars"];
    for (NSUInteger index = 0; index < identifiers.count; index++) {
        NSMenuItem *item = nil;
        for (NSMenuItem *candidate in application.itemArray) {
            if ([candidate.identifier isEqualToString:identifiers[index]]) {
                item = candidate;
                break;
            }
        }
        if (!item) {
            item = commandItem(
                titles[index + 1],
                index == 0 ? LC_EVENT_OPEN_SETTINGS : LC_EVENT_OPEN_ACCOUNTS,
                target,
                index == 0 ? @"," : @"");
            item.identifier = identifiers[index];
            [application insertItem:item atIndex:MIN(index + 1, application.numberOfItems)];
        }
        item.title = titles[index + 1];
        item.target = target;
        item.enabled = YES;
    }
}

// Extend only the optional reopen selector; retain the toolkit's delegate and
// lifecycle methods. Recheck this integration when upgrading Slint/winit.
static BOOL dockReopen(id delegate, SEL selector, NSApplication *application, BOOL visible) {
#if LIGHTCAL_NATIVE_SMOKE
    ++smokeReopens;
#endif
    if (notify) notify(LC_EVENT_DOCK_REOPEN);
    return NO; // Lightcal handles restoration; suppress default document creation.
}

static void installDockReopenHandler(id delegate) {
    SEL selector = @selector(applicationShouldHandleReopen:hasVisibleWindows:);
    if (!delegate || [delegate respondsToSelector:selector]) return;
#if OBJC_BOOL_IS_BOOL
    const char *encoding = "B@:@B";
#else
    const char *encoding = "c@:@c";
#endif
    class_addMethod(object_getClass(delegate), selector, (IMP)dockReopen, encoding);
}

@interface LightcalActions : NSObject
@end

@implementation LightcalActions
- (void)action:(id)sender {
    if (notify) notify((int)[sender tag]);
}

- (void)click:(id)sender {
    NSEvent *event = NSApp.currentEvent;
    lastStatusClickTime = NSProcessInfo.processInfo.systemUptime;
    BOOL isRight = (event.type == NSEventTypeRightMouseDown
        || event.type == NSEventTypeRightMouseUp
        || ((event.modifierFlags & NSEventModifierFlagControl) != 0));
    rememberMenuClick();
    if (isRight) {
        NSMenu *menu = buildStatusMenu(menuTitles, menuWeekday, menuShowMonth, updater != nil, self);
        [NSMenu popUpContextMenu:menu withEvent:event forView:status.button];
    } else if (notify) {
        notify(LC_EVENT_TOGGLE);
    }
}
@end

static OSStatus shortcutHandler(EventHandlerCallRef next, EventRef event, void *data) {
    if (notify) notify(LC_EVENT_TOGGLE);
    return noErr;
}

static NSWindow *calendarWindow(void) {
    if (attachedWindow) return attachedWindow;
    for (NSWindow *window in NSApp.windows) {
        if ([window.title isEqualToString:@"Lightcal"]) return window;
    }
    return nil;
}

static BOOL resignKeyFromStatusClick(void) {
    NSTimeInterval now = NSProcessInfo.processInfo.systemUptime;
    return (now - lastStatusClickTime) < 0.08 || pointInsideStatusButton(NSEvent.mouseLocation);
}

void lc_init(void (*callback)(int)) {
    notify = callback;
    actions = [LightcalActions new];
    permissionStore = [EKEventStore new];
    installDockReopenHandler(NSApp.delegate);

    status = [[NSStatusBar systemStatusBar] statusItemWithLength:NSVariableStatusItemLength];
    status.button.target = actions;
    status.button.action = @selector(click:);
    // Respond on mouse-down so clicks toggle immediately and never race with
    // resign-key or global click-outside dismissal.
    [status.button sendActionOn:NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown];
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];

    InstallEventHandler(
        GetApplicationEventTarget(),
        shortcutHandler,
        1,
        &(EventTypeSpec){kEventClassKeyboard, kEventHotKeyPressed},
        NULL,
        NULL);

    NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
    [center addObserverForName:NSWindowDidMoveNotification
                        object:nil
                         queue:NSOperationQueue.mainQueue
                    usingBlock:^(NSNotification *note) {
                        if (note.object == calendarWindow() && !settingsOpen) {
                            schedulePlacement();
                        }
                    }];
    for (NSString *name in @[NSWindowDidResizeNotification, NSWindowDidBecomeKeyNotification]) {
        [center addObserverForName:name
                            object:nil
                             queue:NSOperationQueue.mainQueue
                        usingBlock:^(NSNotification *note) {
                            if (note.object == calendarWindow()) {
                                schedulePlacement();
                            }
                        }];
    }
    [center addObserverForName:NSWindowDidChangeOcclusionStateNotification
                        object:nil
                         queue:NSOperationQueue.mainQueue
                    usingBlock:^(NSNotification *note) {
                        if (note.object == calendarWindow() && notify) {
                            notify(LC_EVENT_VIEW_CHANGED);
                        }
                    }];
    [NSWorkspace.sharedWorkspace.notificationCenter
        addObserverForName:NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification
                    object:nil
                     queue:NSOperationQueue.mainQueue
                usingBlock:^(NSNotification *note) {
                    if (NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion) {
                        lc_cancel_transition();
                    }
                    if (notify) notify(LC_EVENT_VIEW_CHANGED);
                }];
    [center addObserverForName:NSApplicationDidFinishLaunchingNotification
                        object:nil
                         queue:NSOperationQueue.mainQueue
                    usingBlock:^(NSNotification *note) {
                        installDockReopenHandler(NSApp.delegate);
                        syncApplicationCommands(NSApp.mainMenu, menuTitles, actions);
                        // Re-sync after winit's delegate finishes creating NSApp.mainMenu.
                        dispatch_async(dispatch_get_main_queue(), ^{
                            installDockReopenHandler(NSApp.delegate);
                            syncApplicationCommands(NSApp.mainMenu, menuTitles, actions);
                        });
                        schedulePlacement();
                        NSAppleEventDescriptor *event =
                            NSAppleEventManager.sharedAppleEventManager.currentAppleEvent;
                        if ([[event paramDescriptorForKeyword:keyAEPropData]
                                descriptorForKeyword:keyAELaunchedAsLogInItem].booleanValue
                            && notify) {
                            notify(LC_EVENT_DISMISS);
                        }
                    }];
    for (NSString *name in @[
             EKEventStoreChangedNotification,
             NSSystemTimeZoneDidChangeNotification,
             NSCalendarDayChangedNotification,
             NSApplicationDidChangeScreenParametersNotification,
         ]) {
        [center addObserverForName:name
                            object:nil
                             queue:NSOperationQueue.mainQueue
                        usingBlock:^(NSNotification *note) {
                            schedulePlacement();
                            if (notify) notify(LC_EVENT_REFRESH);
                        }];
    }
    [NSWorkspace.sharedWorkspace.notificationCenter
        addObserverForName:NSWorkspaceDidWakeNotification
                    object:nil
                     queue:NSOperationQueue.mainQueue
                usingBlock:^(NSNotification *note) {
                    if (notify) notify(LC_EVENT_REFRESH);
                }];

    // Live system Dark/Light mode switching.
    [NSDistributedNotificationCenter.defaultCenter
        addObserverForName:@"AppleInterfaceThemeChangedNotification"
                    object:nil
                     queue:NSOperationQueue.mainQueue
                usingBlock:^(NSNotification *note) {
                    if (notify) notify(LC_EVENT_THEME_CHANGED);
                }];

    [center addObserverForName:NSWindowDidResignKeyNotification
                        object:nil
                         queue:NSOperationQueue.mainQueue
                    usingBlock:^(NSNotification *note) {
                        if (note.object == calendarWindow()
                            && !settingsOpen
                            && !resignKeyFromStatusClick()
                            && notify) {
                            notify(LC_EVENT_DISMISS);
                        }
                    }];
    [NSEvent addGlobalMonitorForEventsMatchingMask:
                 NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown
                                           handler:^(NSEvent *event) {
                                               NSWindow *window = calendarWindow();
                                               if (!settingsOpen
                                                   && window.visible
                                                   && !pointInsideStatusButton(NSEvent.mouseLocation)
                                                   && notify) {
                                                   notify(LC_EVENT_DISMISS);
                                               }
                                           }];

    NSBundle *framework = [NSBundle bundleWithPath:
        [NSBundle.mainBundle.privateFrameworksPath
            stringByAppendingPathComponent:@"Sparkle.framework"]];
    if ([framework load]) {
        Class cls = NSClassFromString(@"SPUStandardUpdaterController");
        if (cls) {
            updater = ((id (*)(id, SEL, BOOL, id, id))objc_msgSend)(
                [cls alloc],
                @selector(initWithStartingUpdater:updaterDelegate:userDriverDelegate:),
                YES,
                nil,
                nil);
        }
    }
}

// Draw digits optically centered inside `box` using CoreText glyph path bounds
// (cap-height to baseline), avoiding the descender shift of sizeWithAttributes:.
static void drawCenteredDigits(NSString *digits, NSFont *font, NSColor *color, NSRect box) {
    CGContextRef ctx = NSGraphicsContext.currentContext.CGContext;
    if (!ctx) return;
    NSDictionary *attrs = @{
        (id)kCTFontAttributeName: font,
        (id)kCTForegroundColorAttributeName: (id)color.CGColor,
    };
    NSAttributedString *attributed = [[NSAttributedString alloc] initWithString:digits attributes:attrs];
    CTLineRef line = CTLineCreateWithAttributedString((__bridge CFAttributedStringRef)attributed);
    if (!line) return;
    CGRect bounds = CTLineGetBoundsWithOptions(line, kCTLineBoundsUseGlyphPathBounds);
    CGFloat x = NSMidX(box) - CGRectGetMidX(bounds);
    CGFloat y = NSMidY(box) - CGRectGetMidY(bounds);
    CGContextSaveGState(ctx);
    CGContextSetTextMatrix(ctx, CGAffineTransformIdentity);
    CGContextSetTextPosition(ctx, x, y);
    CTLineDraw(line, ctx);
    CGContextRestoreGState(ctx);
    CFRelease(line);
}

// Menu bar icon: 18×18 pt matching Lightcal's calendar-page artwork (top header band
// + optically centered bold SF Pro date). Monochrome uses a true macOS template image
// so AppKit adapts automatically to wallpaper vibrancy, Dark Mode and inactive displays.
static NSImage *menuBarDateImage(int day, BOOL mono) {
    const CGFloat side = 18.0;
    NSImage *image = [NSImage imageWithSize:NSMakeSize(side, side)
                                    flipped:NO
                             drawingHandler:^BOOL(NSRect rect) {
        NSString *digits = [NSString stringWithFormat:@"%d", day];
        if (mono) {
            NSColor *ink = NSColor.blackColor;
            NSRect frame = NSInsetRect(rect, 1.25, 1.25);
            CGFloat radius = 3.25;
            NSBezierPath *clip = [NSBezierPath bezierPathWithRoundedRect:NSInsetRect(rect, 0.75, 0.75)
                                                                 xRadius:3.5
                                                                 yRadius:3.5];
            [NSGraphicsContext saveGraphicsState];
            [clip addClip];
            [ink setFill];
            CGFloat bandHeight = 4.0;
            NSRectFill(NSMakeRect(0.0, side - bandHeight - 0.75, side, bandHeight + 1.0));
            [NSGraphicsContext restoreGraphicsState];

            NSBezierPath *outline = [NSBezierPath bezierPathWithRoundedRect:frame
                                                                    xRadius:radius
                                                                    yRadius:radius];
            outline.lineWidth = 1.25;
            [ink setStroke];
            [outline stroke];

            NSFont *font = [NSFont systemFontOfSize:(day < 10 ? 10.5 : 9.25) weight:NSFontWeightBold];
            NSRect body = NSMakeRect(1.5, 1.75, side - 3.0, side - bandHeight - 2.75);
            drawCenteredDigits(digits, font, ink, body);
        } else {
            NSRect tileRect = NSInsetRect(rect, 0.75, 0.75);
            NSBezierPath *tile = [NSBezierPath bezierPathWithRoundedRect:tileRect
                                                                 xRadius:3.5
                                                                 yRadius:3.5];
            [NSGraphicsContext saveGraphicsState];
            [tile addClip];
            [[NSColor colorWithSRGBRed:0.843 green:0.220 blue:0.251 alpha:1.0] setFill];
            NSRectFill(tileRect);
            CGFloat bandHeight = 4.0;
            [[NSColor colorWithSRGBRed:0.663 green:0.165 blue:0.192 alpha:1.0] setFill];
            NSRectFill(NSMakeRect(0.0, NSMaxY(tileRect) - bandHeight, side, bandHeight));
            [NSGraphicsContext restoreGraphicsState];

            NSFont *font = [NSFont systemFontOfSize:(day < 10 ? 11.0 : 9.75) weight:NSFontWeightBold];
            NSRect body = NSMakeRect(
                NSMinX(tileRect),
                NSMinY(tileRect) + 0.5,
                NSWidth(tileRect),
                NSHeight(tileRect) - bandHeight - 0.5);
            drawCenteredDigits(digits, font, NSColor.whiteColor, body);
        }
        return YES;
    }];
    image.template = mono;
    return image;
}

// Running Dock icon: 512×512 logical canvas following Apple's macOS icon grid
// (~824/1024 squircle proportion, continuous corner curve, subtle drop shadow,
// Lightcal's darker coral header band and optically centered SF Pro day numeral).
static NSImage *dockDateImage(int day) {
    const CGFloat side = 512.0;
    return [NSImage imageWithSize:NSMakeSize(side, side)
                          flipped:NO
                   drawingHandler:^BOOL(NSRect rect) {
        const CGFloat margin = 50.0; // ~9.8% Apple macOS icon grid margin
        NSRect tileRect = NSInsetRect(rect, margin, margin);
        CGFloat tileSide = NSWidth(tileRect);
        CGFloat radius = round(tileSide * 0.2237);
        NSBezierPath *tile = [NSBezierPath bezierPathWithRoundedRect:tileRect
                                                             xRadius:radius
                                                             yRadius:radius];

        // Soft macOS Dock drop shadow.
        [NSGraphicsContext saveGraphicsState];
        NSShadow *shadow = [NSShadow new];
        shadow.shadowColor = [NSColor colorWithWhite:0.0 alpha:0.28];
        shadow.shadowOffset = NSMakeSize(0.0, -8.0);
        shadow.shadowBlurRadius = 16.0;
        [shadow set];
        [[NSColor colorWithSRGBRed:0.843 green:0.220 blue:0.251 alpha:1.0] setFill];
        [tile fill];
        [NSGraphicsContext restoreGraphicsState];

        // Clipped tile body + darker coral top band.
        [NSGraphicsContext saveGraphicsState];
        [tile addClip];
        NSGradient *bodyGradient = [[NSGradient alloc]
            initWithStartingColor:[NSColor colorWithSRGBRed:0.875 green:0.255 blue:0.286 alpha:1.0]
                      endingColor:[NSColor colorWithSRGBRed:0.800 green:0.192 blue:0.224 alpha:1.0]];
        [bodyGradient drawInRect:tileRect angle:-90.0];

        CGFloat bandHeight = round(tileSide * 0.245);
        NSRect bandRect = NSMakeRect(
            NSMinX(tileRect),
            NSMaxY(tileRect) - bandHeight,
            tileSide,
            bandHeight);
        NSGradient *bandGradient = [[NSGradient alloc]
            initWithStartingColor:[NSColor colorWithSRGBRed:0.690 green:0.180 blue:0.208 alpha:1.0]
                      endingColor:[NSColor colorWithSRGBRed:0.635 green:0.149 blue:0.176 alpha:1.0]];
        [bandGradient drawInRect:bandRect angle:-90.0];

        // Crisp subtle seam beneath the top band (use SourceOver so alpha blends over coral).
        [[NSColor colorWithWhite:0.0 alpha:0.12] setFill];
        NSRectFillUsingOperation(
            NSMakeRect(NSMinX(tileRect), NSMinY(bandRect) - 1.5, tileSide, 1.5),
            NSCompositingOperationSourceOver);

        // Subtle inner rim highlight.
        [[NSColor colorWithWhite:1.0 alpha:0.14] setStroke];
        NSBezierPath *rim = [NSBezierPath bezierPathWithRoundedRect:NSInsetRect(tileRect, 1.5, 1.5)
                                                            xRadius:radius - 1.5
                                                            yRadius:radius - 1.5];
        rim.lineWidth = 3.0;
        [rim stroke];
        [NSGraphicsContext restoreGraphicsState];

        NSString *digits = [NSString stringWithFormat:@"%d", day];
        NSFontDescriptor *base = [[NSFont systemFontOfSize:(day < 10 ? 228.0 : 204.0)
                                                    weight:NSFontWeightBold] fontDescriptor];
        NSFontDescriptor *rounded = [base fontDescriptorWithDesign:NSFontDescriptorSystemDesignRounded];
        NSFont *font = [NSFont fontWithDescriptor:(rounded ?: base) size:(day < 10 ? 228.0 : 204.0)];
        NSRect bodyArea = NSMakeRect(
            NSMinX(tileRect),
            NSMinY(tileRect) + 8.0,
            tileSide,
            tileSide - bandHeight - 8.0);
        drawCenteredDigits(digits, font, NSColor.whiteColor, bodyArea);
        return YES;
    }];
}

static void syncUpdaterPreferences(id engine, BOOL automatic, BOOL download) {
    // Setting these properties can reset Sparkle's cycle. UI synchronization is
    // frequent; write only when the actual preference changes.
    if ([[engine valueForKey:@"automaticallyChecksForUpdates"] boolValue] != automatic) {
        [engine setValue:@(automatic) forKey:@"automaticallyChecksForUpdates"];
    }
    if ([[engine valueForKey:@"automaticallyDownloadsUpdates"] boolValue] != download) {
        [engine setValue:@(download) forKey:@"automaticallyDownloadsUpdates"];
    }
}

void lc_update(
    const char *text,
    int day,
    bool mono,
    bool settings,
    bool automatic,
    bool download
) {
    if (!status) return;
    BOOL wasSettings = settingsOpen;
    settingsOpen = settings ? YES : NO;
    if (!settingsOpen && wasSettings) {
        settingsMovedByUser = NO;
    }
    NSWindow *window = calendarWindow();
    if (window) {
        applyWindowMode(window, settingsOpen);
    }

    NSString *title = text ? [NSString stringWithUTF8String:text] : @"";
    if (!title) title = @"";
    status.button.font = [NSFont monospacedDigitSystemFontOfSize:14.0 weight:NSFontWeightMedium];
    if (title.length > 0) {
        // Use plain `title` (not attributedTitle) so NSStatusBarButtonCell applies
        // native menu-bar vibrancy, dark/light wallpaper tinting, and inactive dimming.
        status.button.title = [@" " stringByAppendingString:title];
        status.button.imagePosition = NSImageLeft;
    } else {
        status.button.title = @"";
        status.button.imagePosition = NSImageOnly;
    }

    static int renderedMenuDay = 0;
    static BOOL renderedMenuMono = NO;
    static BOOL menuIconReady = NO;
    if (!menuIconReady || renderedMenuDay != day || renderedMenuMono != (mono ? YES : NO)) {
        status.button.image = menuBarDateImage(day, mono ? YES : NO);
        renderedMenuDay = day;
        renderedMenuMono = mono ? YES : NO;
        menuIconReady = YES;
    }

    // The running Dock icon is independent of bundle-icon caching and always colored.
    static int dockDay = 0;
    if (dockDay != day) {
        NSApp.applicationIconImage = dockDateImage(day);
        [NSApp.dockTile display];
        dockDay = day;
    }

    if (updater) {
        id engine = ((id (*)(id, SEL))objc_msgSend)(updater, @selector(updater));
        syncUpdaterPreferences(engine, automatic ? YES : NO, download ? YES : NO);
    }
}

int lc_visible(void) {
    NSWindow *window = calendarWindow();
    if (!window || !window.visible) return 0;
    return (window.occlusionState & NSWindowOcclusionStateVisible) ? 2 : 1;
}

// Placement never activates or opens a window. A deferred pass runs after Slint
// applies its size constraints, including the first show and monitor changes.
static void placeWindow(void) {
    NSWindow *window = calendarWindow();
    if (!window || !window.visible) return;
    configureWindowChrome(window);

    NSWindow *bar = status.button.window;
    NSScreen *screen = bar.screen ?: window.screen ?: NSScreen.mainScreen;
    if (!screen) return;

    NSRect frame = window.frame;
    if (settingsOpen && settingsMovedByUser) {
        NSScreen *windowScreen = window.screen ?: screen;
        NSRect clamped = clampFrameToArea(frame, windowScreen.visibleFrame);
        if (!NSEqualRects(frame, clamped)) {
            [window setFrame:clamped display:YES];
            [window invalidateShadow];
        }
        return;
    }

    NSRect area = screen.visibleFrame;
    NSRect button = bar
        ? [bar convertRectToScreen:[status.button convertRect:status.button.bounds toView:nil]]
        : NSZeroRect;
    CGFloat menuHeight = MAX(
        NSStatusBar.systemStatusBar.thickness,
        NSMaxY(screen.frame) - NSMaxY(area));
    BOOL validAnchor = bar && menuAnchorValid(button, screen.frame, menuHeight);
    if (validAnchor) {
        rememberedAnchor = button;
        rememberedDisplay = screen.deviceDescription[@"NSScreenNumber"];
    } else if (rememberedDisplay) {
        for (NSScreen *candidate in NSScreen.screens) {
            CGFloat height = MAX(
                NSStatusBar.systemStatusBar.thickness,
                NSMaxY(candidate.frame) - NSMaxY(candidate.visibleFrame));
            if ([candidate.deviceDescription[@"NSScreenNumber"] isEqual:rememberedDisplay]
                && menuAnchorValid(rememberedAnchor, candidate.frame, height)) {
                screen = candidate;
                area = screen.visibleFrame;
                button = rememberedAnchor;
                validAnchor = YES;
                break;
            }
        }
    }

    NSRect target = anchoredFrame(frame, area, button, validAnchor);
    static int diagnostics = 0;
    if (getenv("LIGHTCAL_DEBUG_PLACEMENT") && diagnostics++ < 32) {
        fprintf(
            stderr,
            "Lightcal placement: current=%s screen=%s area=%s item=%s target=%s anchor=%d\n",
            NSStringFromRect(frame).UTF8String,
            NSStringFromRect(screen.frame).UTF8String,
            NSStringFromRect(area).UTF8String,
            NSStringFromRect(button).UTF8String,
            NSStringFromRect(target).UTF8String,
            validAnchor);
    }
    if (!NSEqualRects(frame, target)) {
        [window setFrame:target display:YES];
        [window invalidateShadow];
    }
}

static void schedulePlacement(void) {
    if (placementQueued) return;
    placementQueued = YES;
    dispatch_async(dispatch_get_main_queue(), ^{
        placementQueued = NO;
        placeWindow();
    });
}

void lc_place(void) {
    placeWindow();
    schedulePlacement();
}

void lc_show(void) {
    lc_cancel_transition();
    syncApplicationCommands(NSApp.mainMenu, menuTitles, actions);
    NSWindow *window = calendarWindow();
    if (!window) return;
    configureWindowChrome(window);
    if (window.miniaturized) {
        [window deminiaturize:nil];
    }
    [NSApp activateIgnoringOtherApps:YES];
    placeWindow();
    [window makeKeyAndOrderFront:nil];
    schedulePlacement();
}

#if LIGHTCAL_NATIVE_SMOKE
// Exercise AppKit's real AppleEvent dispatch, not a direct delegate invocation.
// These entry points are absent from normal builds and target this process only.
int lc_smoke_reopen(void) {
    pid_t process = getpid();
    AEAddressDesc address = {typeNull, NULL};
    AppleEvent event = {typeNull, NULL};
    OSStatus result = AECreateDesc(typeKernelProcessID, &process, sizeof(process), &address);
    if (result == noErr) {
        result = AECreateAppleEvent(
            kCoreEventClass,
            kAEReopenApplication,
            &address,
            kAutoGenerateReturnID,
            kAnyTransactionID,
            &event);
    }
    if (result == noErr) {
        result = AESendMessage(&event, NULL, kAENoReply, kAEDefaultTimeout);
    }
    AEDisposeDesc(&event);
    AEDisposeDesc(&address);
    return result;
}

bool lc_smoke_settings(void) {
    NSEvent *key = [NSEvent keyEventWithType:NSEventTypeKeyDown
                                    location:NSZeroPoint
                               modifierFlags:NSEventModifierFlagCommand
                                   timestamp:NSProcessInfo.processInfo.systemUptime
                                windowNumber:calendarWindow().windowNumber
                                     context:nil
                                  characters:@","
                 charactersIgnoringModifiers:@","
                                   isARepeat:NO
                                     keyCode:43];
    return [NSApp.mainMenu performKeyEquivalent:key];
}

char *lc_smoke_state(void) {
    NSWindow *window = calendarWindow();
    NSRect frame = window.frame;
    NSRect area = window.screen.visibleFrame;
    BOOL anchored = window
        && fabs(NSMaxY(frame) - (NSMaxY(area) - 4.0)) < 1.0
        && NSMinX(frame) >= NSMinX(area) + 7.0
        && NSMaxX(frame) <= NSMaxX(area) - 7.0;
    NSDictionary *state = @{
        @"visible": window.visible ? @YES : @NO,
        @"miniaturized": window.miniaturized ? @YES : @NO,
        @"anchored": anchored ? @YES : @NO,
        @"alpha": @(window.alphaValue),
        @"snapshot": transitionSnapshot ? @YES : @NO,
        @"reopens": @(smokeReopens),
        @"delegate": [NSApp.delegate
            respondsToSelector:@selector(applicationShouldHandleReopen:hasVisibleWindows:)]
            ? @YES : @NO,
    };
    NSData *json = [NSJSONSerialization dataWithJSONObject:state options:0 error:nil];
    return strdup([[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding].UTF8String);
}
#endif

// Snapshot the outgoing surface once for Calendar <-> Settings transitions; only
// native window opacity animates so Slint never repaints per animation frame.
void lc_begin_transition(double duration) {
    NSWindow *window = calendarWindow();
    if (!attachedWindow
        || !window.visible
        || NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion
        || duration <= 0.0) {
        return;
    }
    if (transitionSnapshot) return;

    NSView *view = window.contentView;
    if (!view || NSWidth(view.bounds) <= 0.0 || NSHeight(view.bounds) <= 0.0) return;

    NSBitmapImageRep *bitmap = [view bitmapImageRepForCachingDisplayInRect:view.bounds];
    if (bitmap) {
        [view cacheDisplayInRect:view.bounds toBitmapImageRep:bitmap];
    } else if (view.layer) {
        CGFloat scale = window.backingScaleFactor;
        bitmap = [[NSBitmapImageRep alloc]
            initWithBitmapDataPlanes:NULL
                          pixelsWide:ceil(NSWidth(view.bounds) * scale)
                          pixelsHigh:ceil(NSHeight(view.bounds) * scale)
                       bitsPerSample:8
                     samplesPerPixel:4
                            hasAlpha:YES
                            isPlanar:NO
                      colorSpaceName:NSDeviceRGBColorSpace
                         bytesPerRow:0
                        bitsPerPixel:0];
        if (bitmap) {
            bitmap.size = view.bounds.size;
            NSGraphicsContext *context = [NSGraphicsContext graphicsContextWithBitmapImageRep:bitmap];
            CGContextScaleCTM(context.CGContext, scale, scale);
            [view.layer renderInContext:context.CGContext];
        }
    }
    if (!bitmap) return;

    ++transitionVersion;
    transitionDuration = duration;
    transitionScheduled = NO;

    NSImage *image = [[NSImage alloc] initWithSize:view.bounds.size];
    [image addRepresentation:bitmap];

    transitionSnapshot = [[NSPanel alloc]
        initWithContentRect:window.frame
                  styleMask:NSWindowStyleMaskBorderless | NSWindowStyleMaskNonactivatingPanel
                    backing:NSBackingStoreBuffered
                      defer:NO];
    transitionSnapshot.releasedWhenClosed = NO;
    transitionSnapshot.ignoresMouseEvents = YES;
    transitionSnapshot.opaque = NO;
    transitionSnapshot.backgroundColor = NSColor.clearColor;
    transitionSnapshot.hasShadow = YES;
    transitionSnapshot.level = MAX(window.level, NSStatusWindowLevel);
    transitionSnapshot.animationBehavior = NSWindowAnimationBehaviorNone;
    applyWindowAppearance(transitionSnapshot);

    NSImageView *copy = [[NSImageView alloc] initWithFrame:view.bounds];
    copy.image = image;
    copy.imageScaling = NSImageScaleAxesIndependently;
    copy.wantsLayer = YES;
    copy.layer.cornerRadius = 8.0;
    if (@available(macOS 10.15, *)) {
        copy.layer.cornerCurve = kCACornerCurveContinuous;
    }
    copy.layer.masksToBounds = YES;
    transitionSnapshot.contentView = copy;

    [transitionSnapshot orderWindow:NSWindowAbove relativeTo:window.windowNumber];
    window.alphaValue = 0.0;
}

void lc_finish_transition(void) {
    if (!transitionSnapshot || transitionScheduled) return;
    transitionScheduled = YES;
    NSUInteger version = transitionVersion;
    dispatch_async(dispatch_get_main_queue(), ^{
        NSWindow *window = calendarWindow();
        if (version != transitionVersion) return;
        if (!window.visible
            || NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion) {
            lc_cancel_transition();
            return;
        }
        placeWindow();
        transitionSnapshot.level = MAX(window.level, NSStatusWindowLevel);
        [transitionSnapshot orderWindow:NSWindowAbove relativeTo:window.windowNumber];
        [NSAnimationContext runAnimationGroup:^(NSAnimationContext *context) {
            context.duration = transitionDuration;
            context.timingFunction =
                [CAMediaTimingFunction functionWithControlPoints:0.23 :1.0 :0.32 :1.0];
            window.animator.alphaValue = 1.0;
            transitionSnapshot.animator.alphaValue = 0.0;
        } completionHandler:^{
            if (version == transitionVersion) {
                lc_cancel_transition();
            }
        }];
    });
}

// Map the active macOS key-down event's physical ANSI keyCode so Option+letter
// (which emits diacritics like "ç") and non-Latin keyboard layouts record cleanly.
const char *lc_current_key_name(void) {
    NSEvent *event = NSApp ? NSApp.currentEvent : nil;
    if (!event || event.type != NSEventTypeKeyDown) return NULL;
    switch (event.keyCode) {
        case 0x00: return "A";
        case 0x01: return "S";
        case 0x02: return "D";
        case 0x03: return "F";
        case 0x04: return "H";
        case 0x05: return "G";
        case 0x06: return "Z";
        case 0x07: return "X";
        case 0x08: return "C";
        case 0x09: return "V";
        case 0x0B: return "B";
        case 0x0C: return "Q";
        case 0x0D: return "W";
        case 0x0E: return "E";
        case 0x0F: return "R";
        case 0x10: return "Y";
        case 0x11: return "T";
        case 0x12: return "1";
        case 0x13: return "2";
        case 0x14: return "3";
        case 0x15: return "4";
        case 0x16: return "6";
        case 0x17: return "5";
        case 0x19: return "9";
        case 0x1A: return "7";
        case 0x1C: return "8";
        case 0x1D: return "0";
        case 0x1F: return "O";
        case 0x20: return "U";
        case 0x22: return "I";
        case 0x23: return "P";
        case 0x25: return "L";
        case 0x26: return "J";
        case 0x28: return "K";
        case 0x2D: return "N";
        case 0x2E: return "M";
        case 0x31: return "Space";
        case 0x7A: return "F1";
        case 0x78: return "F2";
        case 0x63: return "F3";
        case 0x76: return "F4";
        case 0x60: return "F5";
        case 0x61: return "F6";
        case 0x62: return "F7";
        case 0x64: return "F8";
        case 0x65: return "F9";
        case 0x6D: return "F10";
        case 0x67: return "F11";
        case 0x6F: return "F12";
        default: return NULL;
    }
}

void lc_open(const char *url) {
    if (!url) return;
    NSString *text = [NSString stringWithUTF8String:url];
    if (!text) return;
    NSURL *parsed = [NSURL URLWithString:text];
    if (parsed) {
        [NSWorkspace.sharedWorkspace openURL:parsed];
    }
}

void lc_calendar(void) {
    [NSWorkspace.sharedWorkspace openURL:[NSURL fileURLWithPath:@"/System/Applications/Calendar.app"]];
}

void lc_permission(void) {
    if (!permissionStore) {
        permissionStore = [EKEventStore new];
    }
    [permissionStore requestFullAccessToEventsWithCompletion:^(BOOL granted, NSError *error) {
        dispatch_async(dispatch_get_main_queue(), ^{
            if (notify) notify(LC_EVENT_REFRESH);
        });
    }];
}

bool lc_dark(void) {
    NSAppearance *appearance = NSApp ? NSApp.effectiveAppearance : nil;
    if (appearance) {
        NSAppearanceName match = [appearance bestMatchFromAppearancesWithNames:@[
            NSAppearanceNameDarkAqua,
            NSAppearanceNameAqua,
        ]];
        if (match) {
            return [match isEqualToString:NSAppearanceNameDarkAqua];
        }
    }
    return [[NSUserDefaults.standardUserDefaults stringForKey:@"AppleInterfaceStyle"]
        isEqualToString:@"Dark"];
}

bool lc_motion(void) {
    return !NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion;
}

int lc_startup(bool enable) {
    NSError *error = nil;
    SMAppService *service = SMAppService.mainAppService;
    BOOL ok = enable
        ? [service registerAndReturnError:&error]
        : [service unregisterAndReturnError:&error];
    return ok && (!enable || service.status == SMAppServiceStatusEnabled) ? 0 : 1;
}

int lc_startup_status(void) {
    return SMAppService.mainAppService.status == SMAppServiceStatusEnabled;
}

bool lc_shortcut(int code, unsigned modifiers) {
    EventHotKeyRef replacement = NULL;
    if (code >= 0
        && RegisterEventHotKey(
               code,
               modifiers,
               (EventHotKeyID){'LCAL', 1},
               GetApplicationEventTarget(),
               0,
               &replacement) != noErr) {
        return false;
    }
    if (hotkey) {
        UnregisterEventHotKey(hotkey);
    }
    hotkey = replacement;
    return true;
}

bool lc_updater_ready(void) {
    return updater != nil;
}

void lc_check_updates(void) {
    if (updater) {
        ((void (*)(id, SEL, id))objc_msgSend)(updater, @selector(checkForUpdates:), nil);
    }
}

void lc_copy(const char *text) {
    if (!text) return;
    NSString *value = [NSString stringWithUTF8String:text];
    if (!value) return;
    [NSPasteboard.generalPasteboard clearContents];
    [NSPasteboard.generalPasteboard setString:value forType:NSPasteboardTypeString];
}

static BOOL midnightInZone(NSDate *date, NSTimeZone *zone, NSCalendar *calendar) {
    calendar.timeZone = zone;
    NSDateComponents *parts =
        [calendar components:NSCalendarUnitHour | NSCalendarUnitMinute | NSCalendarUnitSecond
                    fromDate:date];
    return parts.hour == 0 && parts.minute == 0 && parts.second == 0;
}

// Export civil boundaries before crossing FFI. EventKit floating dates normally
// use the default zone; explicit event zones and UTC-midnight source dates must
// not be shifted into the Rust process's current local day.
static NSDictionary *allDayDatesWithHelpers(
    NSDate *start,
    NSDate *end,
    NSTimeZone *eventZone,
    NSTimeZone *displayZone,
    NSCalendar *calendar,
    NSDateFormatter *formatter,
    NSTimeZone *utc
) {
    if (!start || !end || [end compare:start] != NSOrderedDescending) return nil;
    NSTimeZone *zone = eventZone ?: displayZone;
    if (eventZone
        && midnightInZone(start, eventZone, calendar)
        && midnightInZone(end, eventZone, calendar)) {
        zone = eventZone;
    } else if (midnightInZone(start, displayZone, calendar)
               && midnightInZone(end, displayZone, calendar)) {
        zone = displayZone;
    } else if (midnightInZone(start, utc, calendar)
               && midnightInZone(end, utc, calendar)) {
        zone = utc;
    }
    formatter.timeZone = zone;
    return @{
        @"start_date": [formatter stringFromDate:start],
        @"end_date_exclusive": [formatter stringFromDate:end],
    };
}

static __attribute__((unused)) NSDictionary *allDayDates(
    NSDate *start,
    NSDate *end,
    NSTimeZone *eventZone,
    NSTimeZone *displayZone
) {
    NSCalendar *calendar = [[NSCalendar alloc] initWithCalendarIdentifier:NSCalendarIdentifierGregorian];
    NSDateFormatter *formatter = [NSDateFormatter new];
    formatter.locale = [[NSLocale alloc] initWithLocaleIdentifier:@"en_US_POSIX"];
    formatter.calendar = [[NSCalendar alloc] initWithCalendarIdentifier:NSCalendarIdentifierGregorian];
    formatter.dateFormat = @"yyyy-MM-dd";
    NSTimeZone *utc = [NSTimeZone timeZoneForSecondsFromGMT:0];
    return allDayDatesWithHelpers(start, end, eventZone, displayZone, calendar, formatter, utc);
}

// Called only on a background worker. Reuse one persistent worker EKEventStore
// (per Apple EventKit guidance) instead of reconnecting to calaccessd per query.
char *lc_events(double from, double to) {
    @autoreleasepool {
        static EKEventStore *workerStore = nil;
        static NSObject *storeLock = nil;
        static dispatch_once_t once;
        dispatch_once(&once, ^{
            storeLock = [NSObject new];
        });

        EKAuthorizationStatus authorization =
            [EKEventStore authorizationStatusForEntityType:EKEntityTypeEvent];
        if (authorization != EKAuthorizationStatusFullAccess) {
            @synchronized(storeLock) {
                workerStore = nil;
            }
            if (authorization == EKAuthorizationStatusNotDetermined) {
                return strdup("{\"denied\":false,\"sources\":[],\"events\":[]}");
            }
            return strdup("{\"denied\":true,\"sources\":[],\"events\":[]}");
        }

        NSMutableArray *sources = [NSMutableArray new];
        NSMutableArray *events = [NSMutableArray new];

        @synchronized(storeLock) {
            if (!workerStore) {
                workerStore = [EKEventStore new];
            }
            NSArray *calendars = [workerStore calendarsForEntityType:EKEntityTypeEvent];
            for (EKCalendar *calendar in calendars) {
                NSColor *color = calendar.CGColor
                    ? [[NSColor colorWithCGColor:calendar.CGColor]
                          colorUsingColorSpace:NSColorSpace.sRGBColorSpace]
                    : nil;
                NSString *hex = color
                    ? [NSString stringWithFormat:@"#%02X%02X%02X",
                          (int)lround(color.redComponent * 255.0),
                          (int)lround(color.greenComponent * 255.0),
                          (int)lround(color.blueComponent * 255.0)]
                    : @"#008573";
                [sources addObject:@{
                    @"id": calendar.calendarIdentifier ?: @"",
                    @"name": calendar.title ?: @"",
                    @"account": calendar.source.title ?: @"macOS",
                    @"color": hex,
                }];
            }

            if (calendars.count > 0) {
                NSCalendar *gregorian =
                    [[NSCalendar alloc] initWithCalendarIdentifier:NSCalendarIdentifierGregorian];
                NSDateFormatter *formatter = [NSDateFormatter new];
                formatter.locale = [[NSLocale alloc] initWithLocaleIdentifier:@"en_US_POSIX"];
                formatter.calendar =
                    [[NSCalendar alloc] initWithCalendarIdentifier:NSCalendarIdentifierGregorian];
                formatter.dateFormat = @"yyyy-MM-dd";
                NSTimeZone *utc = [NSTimeZone timeZoneForSecondsFromGMT:0];
                NSTimeZone *displayZone = NSTimeZone.defaultTimeZone;

                NSPredicate *predicate = [workerStore
                    predicateForEventsWithStartDate:[NSDate dateWithTimeIntervalSince1970:from]
                                            endDate:[NSDate dateWithTimeIntervalSince1970:to]
                                          calendars:calendars];
                for (EKEvent *event in [workerStore eventsMatchingPredicate:predicate]) {
                    if (events.count >= 20000) break;
                    NSMutableDictionary *row = [@{
                        @"id": event.eventIdentifier ?: @"",
                        @"source": event.calendar.calendarIdentifier ?: @"",
                        @"title": event.title ?: @"",
                        @"location": event.location ?: @"",
                        @"url": event.URL.absoluteString ?: @"",
                        @"all_day": event.allDay ? @YES : @NO,
                        @"start": @(event.startDate.timeIntervalSince1970),
                        @"end": @(event.endDate.timeIntervalSince1970),
                    } mutableCopy];
                    if (event.allDay) {
                        NSDictionary *dates = allDayDatesWithHelpers(
                            event.startDate,
                            event.endDate,
                            event.timeZone,
                            displayZone,
                            gregorian,
                            formatter,
                            utc);
                        if (!dates) continue;
                        [row addEntriesFromDictionary:dates];
                    }
                    [events addObject:row];
                }
            }
        }

        NSData *data = [NSJSONSerialization
            dataWithJSONObject:@{@"denied": @NO, @"sources": sources, @"events": events}
                       options:0
                         error:nil];
        if (!data) return NULL;
        return strdup([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String);
    }
}

void lc_free(char *value) {
    free(value);
}

// OAuth credentials live in this app's generic-password item, never preferences.
// These blocking Keychain operations are called only by provider workers.
static NSDictionary *credentialQuery(void) {
    return @{
        (__bridge id)kSecClass: (__bridge id)kSecClassGenericPassword,
        (__bridge id)kSecAttrService: @"app.koona.lightcal.accounts",
        (__bridge id)kSecAttrAccount: @"default",
        (__bridge id)kSecAttrSynchronizable: @NO,
    };
}

int lc_credentials_read(char **output) {
    @autoreleasepool {
        *output = NULL;
        NSMutableDictionary *query = [credentialQuery() mutableCopy];
        query[(__bridge id)kSecReturnData] = @YES;
        query[(__bridge id)kSecMatchLimit] = (__bridge id)kSecMatchLimitOne;
        CFTypeRef result = NULL;
        OSStatus code = SecItemCopyMatching((__bridge CFDictionaryRef)query, &result);
        if (code == errSecItemNotFound) return 0;
        if (code != errSecSuccess) return code;
        NSData *data = CFBridgingRelease(result);
        if (data.length > 1024 * 1024) return errSecDecode;
        char *bytes = malloc(data.length + 1);
        if (!bytes) return errSecAllocate;
        memcpy(bytes, data.bytes, data.length);
        bytes[data.length] = 0;
        *output = bytes;
        return 0;
    }
}

int lc_credentials_write(const char *json, bool remove) {
    @autoreleasepool {
        NSDictionary *query = credentialQuery();
        if (remove) {
            OSStatus code = SecItemDelete((__bridge CFDictionaryRef)query);
            return code == errSecItemNotFound ? 0 : code;
        }
        if (!json) return errSecParam;
        NSString *text = [NSString stringWithUTF8String:json];
        if (!text) return errSecParam;
        NSData *data = [text dataUsingEncoding:NSUTF8StringEncoding];
        NSDictionary *values = @{(__bridge id)kSecValueData: data};
        OSStatus code = SecItemUpdate((__bridge CFDictionaryRef)query, (__bridge CFDictionaryRef)values);
        if (code == errSecItemNotFound) {
            NSMutableDictionary *item = [query mutableCopy];
            item[(__bridge id)kSecValueData] = data;
            item[(__bridge id)kSecAttrLabel] = @"Lightcal calendar accounts";
            code = SecItemAdd((__bridge CFDictionaryRef)item, NULL);
        }
        return code;
    }
}
