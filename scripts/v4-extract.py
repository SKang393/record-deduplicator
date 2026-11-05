"""Extract the authenticated code-only candidate without links or overwrites."""
import os
from pathlib import Path
import shutil
import stat
import sys
import zipfile

archive, output = Path(sys.argv[1]), Path(sys.argv[2])
output.mkdir(parents=False, exist_ok=False)
root = output.resolve(strict=True)
seen = set()
with zipfile.ZipFile(archive) as z:
    entries = z.infolist()
    if len(entries) != 113:
        raise SystemExit('Unexpected source entry count')
    for entry in entries:
        parts = entry.filename.split('/')
        if (len(parts) < 2 or parts[0] != 'source' or
                any(part in {'', '.', '..'} or '\\' in part or ':' in part for part in parts) or
                entry.filename.casefold() in seen or entry.is_dir() or stat.S_ISLNK(entry.external_attr >> 16)):
            raise SystemExit('Unsafe source archive entry')
        seen.add(entry.filename.casefold())
        dest = root.joinpath(*parts[1:])
        parent = root
        for part in parts[1:-1]:
            parent = parent / part
            if parent.is_symlink() or (parent.exists() and not parent.is_dir()):
                raise SystemExit('Unsafe source parent')
            parent.mkdir(exist_ok=True)
        if os.path.lexists(dest):
            raise SystemExit('Source entry already exists')
        with z.open(entry) as src, dest.open('xb') as dst:
            shutil.copyfileobj(src, dst)
