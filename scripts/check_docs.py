"""Check repository Markdown file links without fetching external resources."""

from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
EXCLUDED = {".git", "target", "node_modules", ".venv", ".local"}
broken = []
documents = 0
for path in sorted(ROOT.rglob("*.md")):
    if EXCLUDED.intersection(path.relative_to(ROOT).parts):
        continue
    documents += 1
    content = re.sub(r"```.*?```", "", path.read_text(encoding="utf-8"), flags=re.S)
    for match in re.finditer(r"\[[^\]]*\]\(([^)]+)\)", content):
        target = match.group(1).strip().strip("<>")
        if urlsplit(target).scheme or target.startswith(("#", "//")):
            continue
        relative = unquote(target.split("#", 1)[0])
        if relative and not (path.parent / relative).exists():
            broken.append(f"{path.relative_to(ROOT)}: {target}")

if broken:
    print("Broken local Markdown links:\n" + "\n".join(broken), file=sys.stderr)
    sys.exit(1)
print(f"PASS: local file links in {documents} Markdown documents")
