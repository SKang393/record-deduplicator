"""Inventory original RIS export bytes and record boundaries, without rewriting files.

This is collection-integrity checking, not deduplication or model validation.
"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
from datetime import datetime, timezone


def inspect(path: Path, root: Path) -> dict:
    raw = path.read_bytes()
    warnings = []
    if raw.startswith((b"\xff\xfe", b"\xfe\xff")):
        encoding = "utf-16"
    else:
        encoding = "utf-8-sig"
    try:
        content = raw.decode(encoding)
    except UnicodeDecodeError:
        encoding = "cp1252"
        warnings.append("UTF decoding failed; Windows-1252 fallback used for boundary counting")
        try:
            content = raw.decode(encoding)
        except UnicodeDecodeError as error:
            return {
                "path": path.relative_to(root).as_posix(),
                "bytes": len(raw),
                "sha256": hashlib.sha256(raw).hexdigest(),
                "status": "decode_error",
                "error": str(error),
            }
    starts = ends = 0
    open_record = False
    errors = []
    for line_number, line in enumerate(content.splitlines(), 1):
        if re.match(r"^TY\s{2}-", line):
            starts += 1
            if open_record:
                errors.append(f"Nested TY at line {line_number}")
            open_record = True
        elif re.match(r"^ER\s{2}-", line):
            ends += 1
            if not open_record:
                errors.append(f"ER without TY at line {line_number}")
            open_record = False
    if open_record:
        errors.append("Unclosed final record")
    if starts == 0:
        errors.append("No RIS TY record boundaries found")
    return {
        "path": path.relative_to(root).as_posix(),
        "bytes": len(raw),
        "sha256": hashlib.sha256(raw).hexdigest(),
        "encoding_for_counting": encoding,
        "ty_count": starts,
        "er_count": ends,
        "warnings": warnings,
        "errors": errors,
        "status": "boundaries_ok" if not errors and starts == ends else "boundary_error",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path, help="Directory containing original .ris exports")
    args = parser.parse_args()
    root = args.directory.resolve(strict=True)
    files = sorted(p for p in root.rglob("*") if p.is_file() and p.suffix.lower() == ".ris")
    results = [inspect(path, root) for path in files]
    report = {
        "checked_at_utc": datetime.now(timezone.utc).isoformat(),
        "scope": "Original bytes, SHA-256, encoding and TY/ER boundaries only",
        "file_count": len(results),
        "record_count": sum(r.get("ty_count", 0) for r in results),
        "files": results,
    }
    print(json.dumps(report, indent=2, ensure_ascii=False))
    if not results:
        print("INCOMPLETE: no RIS exports present", file=sys.stderr)
        return 2
    return 0 if all(r["status"] == "boundaries_ok" for r in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
