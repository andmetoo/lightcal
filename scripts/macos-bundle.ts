import { mkdir, rm } from "node:fs/promises";
import { join } from "node:path";

/** Assemble each build from an empty bundle, never a previous signed payload. */
export async function prepareMacBundle(app: string): Promise<void> {
  await rm(app, { recursive: true, force: true });
  await mkdir(join(app, "Contents/MacOS"), { recursive: true });
  await mkdir(join(app, "Contents/Resources"), { recursive: true });
}
