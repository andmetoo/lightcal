import { expect, test } from "bun:test";
import { mkdtemp, mkdir, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { prepareMacBundle } from "./macos-bundle";

test("review packaging removes stale signing and Sparkle files from a previous release", async () => {
  const directory = await mkdtemp(join(tmpdir(), "lightcal-bundle-"));
  const app = join(directory, "Lightcal.app");
  try {
    await mkdir(join(app, "Contents/Frameworks/Sparkle.framework"), { recursive: true });
    await mkdir(join(app, "Contents/_CodeSignature"), { recursive: true });
    await writeFile(join(app, "Contents/Frameworks/Sparkle.framework/Sparkle"), "old framework");
    await writeFile(join(app, "Contents/_CodeSignature/CodeResources"), "old signature");
    await writeFile(join(directory, "keep.txt"), "unrelated output");
    await prepareMacBundle(app);
    expect((await readdir(join(app, "Contents"))).sort()).toEqual(["MacOS", "Resources"]);
    expect(await Bun.file(join(directory, "keep.txt")).text()).toBe("unrelated output");
    await writeFile(join(app, "Contents/MacOS/lightcal"), "old executable");
    await prepareMacBundle(app);
    expect(await readdir(join(app, "Contents/MacOS"))).toEqual([]);
  } finally { await rm(directory, { recursive: true, force: true }); }
});
