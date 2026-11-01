/** Shared GitHub CLI checks for the Bun release scripts. */

/** Throws an actionable error when gh is missing or not signed in. */
export function ensureGh(cwd: string): void {
  let signedIn: boolean;
  try {
    signedIn = Bun.spawnSync(["gh", "auth", "status"], { cwd, stdout: "pipe", stderr: "pipe" }).exitCode === 0;
  } catch {
    throw new Error("gh is not installed. Install GitHub CLI (https://cli.github.com) and run gh auth login.");
  }
  if (!signedIn) throw new Error("gh is not signed in. Run gh auth login, then repeat this command.");
}
