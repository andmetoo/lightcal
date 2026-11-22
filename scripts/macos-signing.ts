/** Developer ID codesign commands for the release bundle, kept separate so tests can check them. */
import { join } from "node:path";

/** Entitlement the hardened runtime requires before EventKit may read calendars. */
export const CALENDARS_ENTITLEMENT = "com.apple.security.personal-information.calendars";

const sign = (identity: string) => ["codesign", "--force", "--timestamp", "--options", "runtime", "--sign", identity];

/**
 * Sparkle's nested code, innermost first, as Sparkle's signing guide orders it.
 * Downloader.xpc keeps its own sandbox entitlements; re-signing must not strip them.
 */
export function sparkleSigningCommands(framework: string, identity: string): string[][] {
  const versions = join(framework, "Versions/B");
  return [
    [...sign(identity), join(versions, "XPCServices/Installer.xpc")],
    [...sign(identity), "--preserve-metadata=entitlements", join(versions, "XPCServices/Downloader.xpc")],
    [...sign(identity), join(versions, "Autoupdate")],
    [...sign(identity), join(versions, "Updater.app")],
    [...sign(identity), framework],
  ];
}

/** Signs the enclosing application last, with Lightcal's hardened-runtime entitlements. */
export function appSigningCommand(app: string, identity: string, entitlements: string): string[] {
  return [...sign(identity), "--entitlements", entitlements, app];
}

/** Throws unless `codesign -d --entitlements - --xml` output sets calendar access to true. */
export function assertCalendarsEntitlement(codesignOutput: string): void {
  const granted = new RegExp(`<key>${CALENDARS_ENTITLEMENT.replaceAll(".", "\\.")}</key>\\s*<true\\s*/>`);
  if (!granted.test(codesignOutput)) {
    throw new Error(`Signed app lacks ${CALENDARS_ENTITLEMENT}; EventKit would be denied under hardened runtime.`);
  }
}
