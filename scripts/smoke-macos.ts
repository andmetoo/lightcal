// Run only on a logged-in Mac. This uses native state and self-targeted AppleEvents,
// with synthetic calendars and disposable data; it does not capture the screen.
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

if (process.platform !== "darwin") throw new Error("Native lifecycle checks require macOS.");
const binary = resolve(process.argv[2] ?? "target/debug/lightcal");
const directory = await mkdtemp(join(tmpdir(), "lightcal-native-smoke-"));
try {
  const child = Bun.spawn([binary, "--smoke-native", "--synthetic-calendars", "--background"], {
    env: { ...process.env, LIGHTCAL_DATA_DIR: directory, SLINT_BACKEND: "winit-software" },
    stdout: "pipe", stderr: "pipe",
  });
  const deadline = setTimeout(() => child.kill(), 20_000);
  try {
    const [code, stdout, stderr] = await Promise.all([
      child.exited, new Response(child.stdout).text(), new Response(child.stderr).text(),
    ]);
    process.stdout.write(stdout);
    if (stderr) process.stderr.write(stderr);
    if (code !== 0 || !stdout.includes('native-smoke {"passed":true}')) {
      throw new Error(`Native lifecycle check did not pass (exit ${code}).`);
    }
  } finally { clearTimeout(deadline); }
} finally { await rm(directory, { recursive: true, force: true }); }
