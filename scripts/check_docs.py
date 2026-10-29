"""Check repository Markdown links and whitespace using only Python's stdlib."""

from pathlib import Path
import re
import subprocess
from urllib.parse import unquote, urlsplit


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    # Include untracked planning files while respecting ignore rules.
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=root, check=True, capture_output=True,
    )
    paths = sorted({p.decode("utf-8") for p in result.stdout.split(b"\0") if p})
    errors = []
    checked = 0
    for name in paths:
        path = root / name
        if path.suffix.lower() != ".md" or not path.is_file():
            continue
        checked += 1
        content = path.read_text(encoding="utf-8")
        for number, line in enumerate(content.splitlines(), 1):
            if line != line.rstrip():
                errors.append(f"{name}:{number}: trailing whitespace")
            for target in re.findall(r"!?\[[^\]]*\]\(([^)]+)\)", line):
                target = target.strip().strip("<>")
                parsed = urlsplit(target)
                if parsed.scheme or parsed.netloc or not parsed.path:
                    continue
                destination = (path.parent / unquote(parsed.path)).resolve()
                if not destination.is_relative_to(root):
                    errors.append(f"{name}:{number}: link leaves repository: {target}")
                elif not destination.exists():
                    errors.append(f"{name}:{number}: missing link target: {target}")
        if content and not content.endswith("\n"):
            errors.append(f"{name}: missing final newline")
    for error in errors:
        print(error)
    print(f"Checked {checked} Markdown files; {len(errors)} errors.")
    print("External URLs and heading anchors are not validated.")
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
