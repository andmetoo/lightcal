#!/usr/bin/env bun
/**
 * Copies Lightcal build configuration into GitHub Actions for the release build.
 *
 *   bun run scripts/github-env.ts             # set every non-empty value
 *   bun run scripts/github-env.ts --dry-run   # show what would be set, without values
 *   bun run scripts/github-env.ts --repo owner/name
 *
 * Values come from the process environment first, then the repository `.env`.
 * Public values become repository variables; the Google client secret becomes an
 * encrypted repository secret, sent to gh on stdin. Empty values are skipped, so
 * an unset key never clears what GitHub already holds. Values are never printed.
 */
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

export type Kind = "variable" | "secret";

export interface Entry {
  key: string;
  kind: Kind;
  /** Returns an error message for a malformed value. */
  check: (value: string) => string | undefined;
}

const GUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const SHA256 = /^[0-9a-f]{64}$/i;

export const ENTRIES: Entry[] = [
  {
    key: "LIGHTCAL_MICROSOFT_CLIENT_ID",
    kind: "variable",
    check: (v) => (GUID.test(v) ? undefined : "expected an Entra Application (client) ID GUID"),
  },
  {
    key: "LIGHTCAL_GOOGLE_CLIENT_ID",
    kind: "variable",
    check: (v) => (v.endsWith(".apps.googleusercontent.com") ? undefined : "expected …apps.googleusercontent.com"),
  },
  {
    key: "LIGHTCAL_UPDATE_CERT_SHA256",
    kind: "variable",
    check: (v) =>
      v.split(",").every((pin) => SHA256.test(pin.trim().replaceAll(":", "")))
        ? undefined
        : "expected one or more comma-separated 64-hex SHA-256 fingerprints",
  },
  {
    key: "LIGHTCAL_GOOGLE_CLIENT_SECRET",
    kind: "secret",
    check: (v) => (v.length >= 16 && !/\s/.test(v) ? undefined : "expected the Google Desktop client secret"),
  },
];

/** Reads `KEY=value` lines the same way build.rs does: comments skipped, quotes trimmed. */
export function parseDotenv(text: string): Map<string, string> {
  const values = new Map<string, string>();
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    const eq = line.indexOf("=");
    if (eq < 0) continue;
    const key = line.slice(0, eq).trim();
    if (values.has(key)) continue;
    values.set(key, line.slice(eq + 1).trim().replace(/^["']|["']$/g, ""));
  }
  return values;
}

export interface Planned {
  entry: Entry;
  value?: string;
  error?: string;
}

/** Process environment wins over `.env`, matching build.rs. */
export function plan(env: Record<string, string | undefined>, dotenv: Map<string, string>): Planned[] {
  return ENTRIES.map((entry) => {
    const value = (env[entry.key]?.trim() || dotenv.get(entry.key) || "").trim();
    if (!value) return { entry };
    return { entry, value, error: entry.check(value) };
  });
}

export function ghArgs(entry: Entry, repo: string, value: string): string[] {
  return entry.kind === "secret"
    ? ["gh", "secret", "set", entry.key, "--repo", repo]
    : ["gh", "variable", "set", entry.key, "--repo", repo, "--body", value];
}

function fail(message: string): never {
  console.error(message);
  process.exit(1);
}

function main(argv: string[]): void {
  const root = resolve(import.meta.dir, "..");
  const dryRun = argv.includes("--dry-run");
  const repoFlag = argv.indexOf("--repo");
  let repo = repoFlag >= 0 ? argv[repoFlag + 1] : undefined;
  const unknown = argv.filter(
    (arg, i) => arg !== "--dry-run" && arg !== "--repo" && !(repoFlag >= 0 && i === repoFlag + 1),
  );
  if (unknown.length > 0 || (repoFlag >= 0 && !repo)) fail("Usage: bun run scripts/github-env.ts [--dry-run] [--repo owner/name]");

  const envPath = join(root, ".env");
  const dotenv = existsSync(envPath) ? parseDotenv(readFileSync(envPath, "utf8")) : new Map<string, string>();
  const planned = plan(process.env, dotenv);

  const invalid = planned.filter((p) => p.error);
  if (invalid.length > 0) {
    fail(invalid.map((p) => `${p.entry.key}: ${p.error}. Nothing was changed.`).join("\n"));
  }

  if (!repo) {
    const view = Bun.spawnSync(["gh", "repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"], { cwd: root });
    if (view.exitCode !== 0) fail("Could not detect the repository. Run gh auth login, or pass --repo owner/name.");
    repo = view.stdout.toString().trim();
  }

  console.log(`${dryRun ? "Dry run for" : "Updating"} ${repo}`);
  let failed = false;
  for (const { entry, value } of planned) {
    if (!value) {
      console.log(`  skip   ${entry.key} (empty; GitHub keeps its current value)`);
      continue;
    }
    if (dryRun) {
      console.log(`  would set ${entry.kind} ${entry.key} (${value.length} chars)`);
      continue;
    }
    const proc = Bun.spawnSync(ghArgs(entry, repo, value), {
      cwd: root,
      stdin: entry.kind === "secret" ? new TextEncoder().encode(value) : "ignore",
      stdout: "pipe",
      stderr: "pipe",
    });
    if (proc.exitCode === 0) {
      console.log(`  set    ${entry.kind} ${entry.key}`);
    } else {
      failed = true;
      console.error(`  FAILED ${entry.kind} ${entry.key}: ${proc.stderr.toString().trim()}`);
    }
  }
  if (failed) process.exit(1);
}

if (import.meta.main) main(process.argv.slice(2));
