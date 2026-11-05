"""Extract authenticated inputs without traversal, links, duplicates, or overwrites."""
import os
import pathlib
import shutil
import stat
import sys
import zipfile

archive, target, kind = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3]
if kind not in {'source', 'overlay'}:
    raise SystemExit('Invalid archive kind')
target.mkdir(parents=True, exist_ok=True)
target = target.resolve(strict=True)
seen = set()
with zipfile.ZipFile(archive) as z:
    for entry in z.infolist():
        name = entry.filename
        parts = name.split('/')
        if not name or name.startswith('/') or '\\' in name or any(part in {'', '.', '..'} for part in parts):
            raise SystemExit('Unsafe archive path')
        if name.casefold() in seen or stat.S_ISLNK(entry.external_attr >> 16):
            raise SystemExit('Duplicate or symbolic link in archive')
        seen.add(name.casefold())
        if kind == 'overlay' and not (name == 'app/src-tauri/tests/permissions.rs' or name.startswith(('crates/dedup-core/tests/', 'crates/dedup-cli/tests/', 'testdata/fixtures/'))):
            raise SystemExit('Overlay contains a non-allowlisted path')
        dest = target.joinpath(*parts)
        if entry.is_dir() or os.path.lexists(dest):
            raise SystemExit('Archive contains a directory entry or overlaps an existing path')
        parent = target
        for part in parts[:-1]:
            parent = parent / part
            if parent.is_symlink() or (parent.exists() and not parent.is_dir()):
                raise SystemExit('Unsafe parent path')
            parent.mkdir(exist_ok=True)
        with z.open(entry) as src, open(dest, 'xb') as out:
            shutil.copyfileobj(src, out)
