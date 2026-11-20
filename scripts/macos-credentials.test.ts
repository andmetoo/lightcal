import { expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// Fake every Keychain operation: this test must never read or change the real Keychain.
(process.platform === "darwin" ? test : test.skip)("macOS credentials use scoped Keychain CRUD without plaintext files", async () => {
  const directory = await mkdtemp(join(tmpdir(), "lightcal-keychain-fixture-"));
  try {
    const source = join(directory, "credentials.m");
    await writeFile(source, `#define SecItemCopyMatching LightcalFixtureCopyMatching
#define SecItemAdd LightcalFixtureAdd
#define SecItemUpdate LightcalFixtureUpdate
#define SecItemDelete LightcalFixtureDelete
#import ${JSON.stringify(resolve(import.meta.dir, "../crates/lightcal/src/platform/macos/native.m"))}
#include <assert.h>
static CFDataRef fixture;
static void check(CFDictionaryRef query) {
    assert(CFEqual(CFDictionaryGetValue(query,kSecClass),kSecClassGenericPassword));
    assert(CFEqual(CFDictionaryGetValue(query,kSecAttrService),CFSTR("app.koona.lightcal.accounts")));
    assert(CFEqual(CFDictionaryGetValue(query,kSecAttrAccount),CFSTR("default")));
    assert(CFEqual(CFDictionaryGetValue(query,kSecAttrSynchronizable),kCFBooleanFalse));
}
OSStatus SecItemCopyMatching(CFDictionaryRef query,CFTypeRef *result) {
    check(query);
    assert(CFEqual(CFDictionaryGetValue(query,kSecReturnData),kCFBooleanTrue));
    if(!fixture) return errSecItemNotFound;
    *result=CFRetain(fixture); return 0;
}
OSStatus SecItemAdd(CFDictionaryRef query,CFTypeRef *result) {
    check(query); assert(!fixture);
    fixture=CFDataCreateCopy(NULL,CFDictionaryGetValue(query,kSecValueData)); return 0;
}
OSStatus SecItemUpdate(CFDictionaryRef query,CFDictionaryRef values) {
    check(query); if(!fixture) return errSecItemNotFound;
    CFRelease(fixture); fixture=CFDataCreateCopy(NULL,CFDictionaryGetValue(values,kSecValueData)); return 0;
}
OSStatus SecItemDelete(CFDictionaryRef query) {
    check(query); if(!fixture) return errSecItemNotFound;
    CFRelease(fixture); fixture=NULL; return 0;
}
int main(void) { @autoreleasepool {
    char *value=NULL;
    assert(lc_credentials_read(&value)==0 && !value);
    const char *first=${JSON.stringify(JSON.stringify({google_refresh_token:"fixture-one"}))};
    assert(lc_credentials_write(first,NO)==0);
    assert(lc_credentials_read(&value)==0 && strcmp(value,first)==0); lc_free(value);
    const char *rotated=${JSON.stringify(JSON.stringify({google_refresh_token:"fixture-two"}))};
    assert(lc_credentials_write(rotated,NO)==0);
    assert(lc_credentials_read(&value)==0 && strcmp(value,rotated)==0); lc_free(value);
    assert(lc_credentials_write("{}",YES)==0);
    assert(lc_credentials_read(&value)==0 && !value);
    assert(lc_credentials_write("{}",YES)==0);
    return 0;
} }
`);
    const binary = join(directory, "credentials");
    const compiler = Bun.spawn(["xcrun", "clang", "-fobjc-arc", "-fblocks", "-mmacosx-version-min=14.0", source, "-o", binary,
      ...["AppKit", "QuartzCore", "EventKit", "ServiceManagement", "Carbon", "Security"].flatMap(name => ["-framework", name])], { stdout: "pipe", stderr: "pipe" });
    expect(await compiler.exited, await new Response(compiler.stderr).text()).toBe(0);
    const child = Bun.spawn([binary], { stdout: "pipe", stderr: "pipe" });
    expect(await child.exited, await new Response(child.stderr).text()).toBe(0);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
