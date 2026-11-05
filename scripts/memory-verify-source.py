"""Check every authenticated candidate input without including generated files."""
import hashlib
from pathlib import Path
import sys
import zipfile

archive, root = Path(sys.argv[1]), Path(sys.argv[2]).resolve(strict=True)
with zipfile.ZipFile(archive) as z:
    entries = z.infolist()
    if len(entries) != 113:
        raise SystemExit('Unexpected input count')
    for entry in entries:
        parts = entry.filename.split('/')
        if len(parts) < 2 or parts[0] != 'source' or any(p in {'', '.', '..'} or '\\' in p for p in parts):
            raise SystemExit('Unsafe input path')
        path = root.joinpath(*parts[1:])
        if not path.is_file() or path.is_symlink() or hashlib.sha256(path.read_bytes()).digest() != hashlib.sha256(z.read(entry)).digest():
            raise SystemExit('Candidate source mismatch: ' + entry.filename)
print('All 113 candidate inputs match the authenticated source archive.')
