"""Exercise a native RIS export and a BibTeX export converted by bibutils 7.2."""
import argparse
import csv
import hashlib
import re
import subprocess
import tempfile
from pathlib import Path


def exercise(root, binary):
    # Published results, a follow-up from the same study, and an unrelated report.
    results = "Canopy glass, shade and seedling counts"
    followup = "Canopy glass and seedling survival the following spring"
    control = "Measuring leaf area with paper grids"
    protocol = "Canopy glass, shade and seedling counts: study protocol"
    catalog_records = [("fieldResults", results, "glass"), ("fieldFollowup", followup, "spring"),
                       ("gridSurvey", control, "grids")]
    catalog = root / "catalog.ris"
    catalog.write_bytes("".join(
        f"TY  - JOUR\r\nID  - {key}\r\nTI  - {title}\r\nAU  - Fen, Uma\r\nAU  - Tern, Ivo\r\n"
        f"PY  - 2022\r\nJO  - Fictional Shore Botany\r\nDO  - 10.0000/{doi}\r\nER  -\r\n"
        for key, title, doi in catalog_records).encode())
    # Two copies, and a distinct protocol carrying the results DOI in error.
    bib_records = [("indexResults", results, "glass"), ("indexProtocol", protocol, "glass"),
                   ("indexGrid", control, "grids")]
    bib = root / "index.bib"
    bib.write_bytes("".join(
        f'@article{{{key},\n  title = "{title}",\n  author = {{Fen, Uma and Tern, Ivo}},\n'
        f'  year = {{2022}},\n  journal = {{Fictional Shore Botany}},\n  doi = {{10.0000/{doi}}},\n'
        '  abstract = "Counts at two sites, first in rain,\n              then in sun."\n}\n'
        for key, title, doi in bib_records).encode())
    originals = {p: p.read_bytes() for p in (catalog, bib)}
    converted = root / "index.ris"
    mods = subprocess.check_output(["bib2xml", "-i", "utf8", bib])
    converted.write_bytes(subprocess.check_output(["xml2ris", "-o", "utf8"], input=mods))
    originals[converted] = converted.read_bytes()
    sources = ["--ris-source", "catalog", catalog, "--ris-source", "index", converted]

    def review(*args):
        return subprocess.check_output([binary, "review-ids", *args, *sources], cwd=root).decode()

    sheet = root / "choices.tsv"
    queue = review("--init-decisions", sheet).splitlines()
    groups = [line.split("\t")[2] for line in queue if line.startswith("candidate\tdoi\t")]
    assert sorted(groups) == ["catalog:1,index:1,index:2", "catalog:3,index:3"]
    pair = review("--inspect-pair", sheet, "index:1", "index:2")
    for title in (results, protocol):
        assert f"  TI  - {title}" in pair.splitlines()
    assert "  AB  - Counts at two sites, first in rain, then in sun." in pair.splitlines()
    review("--export-reviewed", sheet, "pending")
    assert "retained=6;suppressed=0" in (root / "pending/mapping.tsv").read_text()

    # Accept only the results copy. Keep the protocol and defer the grid pair.
    edits = {"catalog:1": "keep", "index:1": "duplicate-of=catalog:1", "index:2": "keep"}
    contents = sheet.read_bytes()
    for key, choice in edits.items():
        pending = f"\n{key}\tpending\n".encode()
        assert contents.count(pending) == 1
        contents = contents.replace(pending, f"\n{key}\t{choice}\n".encode())
    sheet.write_bytes(contents)
    review("--export-reviewed", sheet, "reviewed")
    review("--export-reviewed", sheet, "replay")
    for name in ("reviewed.ris", "mapping.tsv", "reconciliation.tsv"):
        assert (root / "reviewed" / name).read_bytes() == (root / "replay" / name).read_bytes()

    mapping = (root / "reviewed/mapping.tsv").read_text().splitlines()
    # Join every source span to its native RIS ID or original BibTeX citation key.
    expected = {"catalog": (catalog, catalog_records, [1, 2, 3]), "index": (converted, bib_records, [1, 4, 5])}
    mapped = {key: (status, detail) for kind, status, key, detail in
              (line.split("\t") for line in mapping) if kind == "map"}
    assert len(mapped) == 6
    for label, (path, records, outputs) in expected.items():
        digest = hashlib.sha256(originals[path]).hexdigest()
        assert f"input\tris\t{label}\tbytes={len(originals[path])};occurrences=3;sha256={digest}" in mapping
        for ordinal, ((key, _, _), output) in enumerate(zip(records, outputs), 1):
            occurrence = f"{label}:{ordinal}"
            status, detail = mapped[occurrence]
            a, b = re.fullmatch(rf"bytes=(\d+)\.\.(\d+);output={output}", detail).groups()
            assert status == edits.get(occurrence, "not-candidate" if occurrence == "catalog:2" else "pending")
            assert f"ID  - {key}" in originals[path][int(a):int(b)].decode("utf-8-sig").splitlines()
    retained = (root / "reviewed/reviewed.ris").read_text().splitlines()
    assert [line[6:] for line in retained if line.startswith("ID  - ")] == [
        "fieldResults", "fieldFollowup", "gridSurvey", "indexProtocol", "indexGrid"]
    assert f"TI  - {protocol}" in retained and f"TI  - {followup}" in retained
    with (root / "reviewed/reconciliation.tsv").open(newline="") as f:
        total, = [row for row in csv.DictReader(f, delimiter="\t") if row["scope"] == "total"]
    assert [total[k] for k in ("input", "removed_id", "removed_metadata", "retained", "pending_id")] == [
        "6", "1", "0", "5", "2"]
    assert all(path.read_bytes() == data for path, data in originals.items())
    print("PASS: 3 RIS + 3 converted BibTeX, 6 retained pending, 1 explicit removal, 5 retained,"
          " 2 pending. Source spans, citation keys, hashes and replay agree.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as tmp:
        exercise(Path(tmp), args.binary.resolve())
