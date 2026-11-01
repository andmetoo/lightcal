#!/usr/bin/env bun
/**
 * Copies Lightcal build configuration into GitHub Actions for the release build.
 *
 *   just github-env [--dry-run] [--repo owner/name]
 *   bun --no-env-file run scripts/github-env.ts [--dry-run] [--repo owner/name]
 *
 * Values are resolved exactly as crates/lightcal/build.rs does: the process environment
 * first (an explicitly empty value wins), then the repository `.env`. `--no-env-file` is
 * required so Bun does not inject `.env.local` or `.env.<NODE_ENV>` values the build never
 * reads. Public values become repository variables; the Google client secret becomes an
 * encrypted repository secret, sent to gh on stdin. Each value is checked with the app's
 * own validation rules. Empty values are skipped, so an unset key never clears what
 * GitHub already holds. Values are never printed.
 */
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { ensureGh } from "./gh.ts";

export type Kind = "variable" | "secret";

export interface Entry {
  key: string;
  kind: Kind;
  /** Returns an error message when the app would reject the value. */
  check: (value: string) => string | undefined;
}

/** Mirrors `calendars::microsoft::valid_client_id`. */
export function validMicrosoftClientId(raw: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(raw);
}

/** Mirrors `calendars::google::valid_client_id`. */
export function validGoogleClientId(raw: string): boolean {
  const suffix = ".apps.googleusercontent.com";
  if (!raw.endsWith(suffix)) return false;
  const left = raw.slice(0, -suffix.length);
  return left.includes("-") && left.length > 8 && /^[A-Za-z0-9_-]+$/.test(left);
}

/** Mirrors `calendars::google::valid_client_secret`. */
export function validGoogleClientSecret(raw: string): boolean {
  const value = raw.trim();
  return value.length >= 10 && value.length < 256 && /^[\x21-\x7e]+$/.test(value) && !/["']/.test(value);
}

/** Mirrors `installer::parse_pins` / `normalize_pin`. */
export function validCertificatePins(raw: string): boolean {
  return raw.split(",").every((pin) => /^[0-9a-f]{64}$/i.test(pin.replace(/[ :]/g, "")));
}

export const ENTRIES: Entry[] = [
  {
    key: "LIGHTCAL_MICROSOFT_CLIENT_ID",
    kind: "variable",
    check: (v) => (validMicrosoftClientId(v) ? undefined : "expected an Entra Application (client) ID GUID"),
  },
  {
    key: "LIGHTCAL_GOOGLE_CLIENT_ID",
    kind: "variable",
    check: (v) =>
      validGoogleClientId(v) ? undefined : "expected a Google Desktop client ID such as 1234567890-abc.apps.googleusercontent.com",
  },
  {
    key: "LIGHTCAL_UPDATE_CERT_SHA256",
    kind: "variable",
    check: (v) =>
      validCertificatePins(v) ? undefined : "expected one or more comma-separated 64-hex SHA-256 fingerprints",
  },
  {
    key: "LIGHTCAL_GOOGLE_CLIENT_SECRET",
    kind: "secret",
    check: (v) =>
      validGoogleClientSecret(v)
        ? undefined
        : "expected the Google Desktop client secret (10-255 printable ASCII characters, no quotes)",
  },
];

/** Rust `str::trim_matches` for one character: strips every leading and trailing copy. */
function trimMatches(value: string, ch: string): string {
  let start = 0;
  let end = value.length;
  while (start < end && value[start] === ch) start++;
  while (end > start && value[end - 1] === ch) end--;
  return value.slice(start, end);
}

/** Reads `.env` the way build.rs does: first match wins, comments skipped, quotes trimmed. */
export function parseDotenv(text: string): Map<string, string> {
  const values = new Map<string, string>();
  for (const raw of text.split(/\r?\n/)) {
    if (raw.trimStart().startsWith("#")) continue;
    const line = raw.trim();
    const eq = line.indexOf("=");
    if (eq < 0) continue;
    const key = line.slice(0, eq).trim();
    if (values.has(key)) continue;
    values.set(key, trimMatches(trimMatches(line.slice(eq + 1).trim(), '"'), "'"));
  }
  return values;
}

export interface Planned {
  entry: Entry;
  value?: string;
  error?: string;
}

/** Process environment (even when empty) wins over `.env`, matching build.rs. */
export function plan(env: Record<string, string | undefined>, dotenv: Map<string, string>): Planned[] {
  return ENTRIES.map((entry) => {
    const value = env[entry.key] ?? dotenv.get(entry.key) ?? "";
    if (!value) return { entry };
    return { entry, value, error: entry.check(value) };
  });
}

export interface Options {
  dryRun: boolean;
  repo?: string;
}

const USAGE = "Usage: just github-env [--dry-run] [--repo owner/name]";

export function parseArgs(argv: string[]): Options {
  const options: Options = { dryRun: false };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--dry-run") {
      options.dryRun = true;
    } else if (arg === "--repo") {
      const repo = argv[++i];
      if (!repo || !/^[\w.-]+\/[\w.-]+$/.test(repo)) throw new Error(`--repo needs owner/name. ${USAGE}`);
      options.repo = repo;
    } else {
      throw new Error(`Unknown argument ${arg}. ${USAGE}`);
    }
  }
  return options;
}

export function ghArgs(entry: Entry, repo: string, value: string): string[] {
  return entry.kind === "secret"
    ? ["gh", "secret", "set", entry.key, "--repo", repo]
    : ["gh", "variable", "set", entry.key, "--repo", repo, "--body", value];
}

function main(argv: string[]): void {
  const root = resolve(import.meta.dir, "..");
  if (!process.execArgv.includes("--no-env-file")) {
    throw new Error(
      "Run this with `just github-env` or `bun --no-env-file run scripts/github-env.ts`, " +
        "so Bun does not load .env.local or similar files that the build never reads.",
    );
  }
  const options = parseArgs(argv);

  const envPath = join(root, ".env");
  const dotenv = existsSync(envPath) ? parseDotenv(readFileSync(envPath, "utf8")) : new Map<string, string>();
  const planned = plan(process.env, dotenv);

  const invalid = planned.filter((p) => p.error);
  if (invalid.length > 0) {
    throw new Error(invalid.map((p) => `${p.entry.key}: ${p.error}.`).join("\n") + "\nNothing was changed.");
  }

  ensureGh(root);
  let repo = options.repo;
  if (!repo) {
    const view = Bun.spawnSync(["gh", "repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"], {
      cwd: root,
      stdout: "pipe",
      stderr: "pipe",
    });
    repo = view.exitCode === 0 ? view.stdout.toString().trim() : "";
    if (!repo) throw new Error("Could not detect the repository. Pass --repo owner/name.");
  }

  console.log(`${options.dryRun ? "Dry run for" : "Updating"} ${repo}`);
  let failed = false;
  for (const { entry, value } of planned) {
    if (!value) {
      console.log(`  skip   ${entry.key} (empty; GitHub keeps its current value)`);
      continue;
    }
    if (options.dryRun) {
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

if (import.meta.main) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
