/** Select the native installer without evaluating arguments through a shell. */
import { resolve } from "node:path";

export function installerCommand(platform: string, args: string[]): string[] {
  if (platform === "win32") {
    return ["pwsh", "-NoLogo", "-NoProfile", "-File", "scripts/package-nsis.ps1", ...args];
  }
  if (platform === "darwin") {
    return ["bun", "scripts/package-macos.ts", "--dmg", ...args];
  }
  throw new Error("Installers can be built on macOS or Windows. Use just check on Linux.");
}

if (import.meta.main) {
  try {
    const child = Bun.spawn(installerCommand(process.platform, process.argv.slice(2)), {
      cwd: resolve(import.meta.dir, ".."), env: process.env,
      stdio: ["inherit", "inherit", "inherit"],
    });
    process.exit(await child.exited);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
