// Check repository Markdown links, trailing whitespace and final newlines.
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, isAbsolute, relative, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
// Include untracked files while respecting ignore rules.
const listed = Bun.spawnSync(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], { cwd: root });
if (listed.exitCode !== 0) throw new Error(listed.stderr.toString());
const paths = [...new Set(listed.stdout.toString().split("\0").filter(Boolean))].sort();

const errors: string[] = [];
let checked = 0;
const link = /!?\[[^\]]*\]\(([^)]+)\)/g;
const scheme = /^[a-z][a-z0-9+.-]*:/i;

for (const name of paths) {
  const path = resolve(root, name);
  if (!name.toLowerCase().endsWith(".md") || !existsSync(path) || !statSync(path).isFile()) continue;
  checked++;
  const content = readFileSync(path, "utf8");
  // Windows checkouts may use CRLF; Git stores LF. Only spaces and tabs count.
  content.split(/\r?\n/).forEach((line, index) => {
    const number = index + 1;
    if (line !== line.trimEnd()) errors.push(`${name}:${number}: trailing whitespace`);
    for (const match of line.matchAll(link)) {
      const target = match[1].trim().replace(/^<|>$/g, "");
      if (scheme.test(target) || target.startsWith("//")) continue;
      const local = target.split("#")[0].split("?")[0];
      if (!local) continue;
      const destination = resolve(dirname(path), decodeURIComponent(local));
      const fromRoot = relative(root, destination);
      if (fromRoot.startsWith("..") || isAbsolute(fromRoot)) {
        errors.push(`${name}:${number}: link leaves repository: ${target}`);
      } else if (!existsSync(destination)) {
        errors.push(`${name}:${number}: missing link target: ${target}`);
      }
    }
  });
  if (content && !content.endsWith("\n")) errors.push(`${name}: missing final newline`);
}

for (const error of errors) console.log(error);
console.log(`Checked ${checked} Markdown files; ${errors.length} errors.`);
console.log("External URLs and heading anchors are not validated.");
process.exit(errors.length ? 1 : 0);
