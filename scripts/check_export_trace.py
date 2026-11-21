"""Trace a completed four-source synthetic review to renamed original downloads."""
import argparse
import csv
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path

from check_large_review import planned_review, render, write_corpus


def exercise(directory, binary, baseline):
    inputs = directory / "downloads"
    inputs.mkdir()
    _, copies, _, originals, spans, groups = write_corpus(inputs, studies=5400, render=render)
    _, edits, _ = planned_review(copies)
    flags = [arg for group in groups for arg in group]
    sheet = directory / "choices.tsv"

    def review(cli, *args):
        subprocess.run([cli, "review-ids", *args, *flags], cwd=directory,
                       stdout=subprocess.DEVNULL, check=True)

    review(baseline, "--init-metadata-decisions", sheet)
    rows = []
    for line in sheet.read_text(encoding="utf-8").splitlines():
        key, _, value = line.partition("\t")
        rows.append(key + "\t" + edits[key] if value == "pending" and key in edits else line)
    sheet.write_text("\n".join(rows) + "\n", encoding="utf-8")
    review(baseline, "--export-reviewed", sheet, "published")
    out = directory / "published"
    if binary != baseline:
        review(binary, "--export-reviewed", sheet, "replay")
        assert all(p.read_bytes() == (directory / "replay" / p.name).read_bytes() for p in out.iterdir())
    with (out / "reconciliation.tsv").open(encoding="utf-8", newline="") as stream:
        counts = list(csv.DictReader(stream, delimiter="\t"))[-1]
    assert (counts["input"], counts["retained"]) == ("15210", "8438")

    # The published mapping is sufficient, even if a subsequent edit changes the sheet.
    sheet.write_text("later sheet", encoding="utf-8")
    archive = directory / "archive"
    archive.mkdir()
    renamed = {"central": archive / "Main database export.ris", "index": inputs / "Index download 02.ris"}
    for label, path in renamed.items():
        (inputs / (label + ".ris")).rename(path)
    # Same filename and byte length as the keeper. Only the source hash distinguishes it.
    wrong = directory / "other"
    wrong.mkdir()
    (wrong / renamed["central"].name).write_bytes(originals["central"].replace(b"JOUR", b"BOOK", 1))
    mapping = out / "mapping.tsv"
    altered = directory / "changed-span.tsv"
    start, end = spans["index:9"]
    old = f"\tindex:9\tbytes={start}..{end};"
    altered.write_text(mapping.read_text(encoding="utf-8").replace(old, old.replace(str(end), str(end + 1)), 1),
                       encoding="utf-8")

    def snapshot():
        return {p.relative_to(directory): hashlib.sha256(p.read_bytes()).hexdigest()
                for p in directory.rglob("*") if p.is_file()}

    before = snapshot()

    def trace(key, folders=("downloads", "archive"), saved=mapping, check=True):
        return subprocess.run([binary, "trace", saved, key, *folders], cwd=directory,
                              capture_output=True, text=True, encoding="utf-8", check=check)

    retained = [key for key in copies if "duplicate-of=" not in edits.get(key, "")]
    for key, target in [("index:9", "central:11"), ("index:11", None)]:
        keys = [key, target] if target else [key]
        number = retained.index(target or key) + 1
        lines = trace(key).stdout.splitlines()
        assert len(lines) == len(keys) + 1
        for line, occurrence in zip(lines[1:], keys):
            label = occurrence.split(":")[0]
            start, end = spans[occurrence]
            expected = [occurrence, edits.get(occurrence, "pending").split("\t")[0], str(number),
                        f"bytes={start}..{end}", hashlib.sha256(originals[label]).hexdigest()]
            fields = line.split("\t")
            assert fields[:-1] == expected and Path(json.loads(fields[-1])) == renamed[label].relative_to(directory)
            print("\t".join(fields[:4]))
    failed = trace("index:9", folders=("downloads", "other"), check=False)
    assert failed.returncode == 2 and not failed.stdout and "central:11" in failed.stderr
    failed = trace("index:9", saved=altered, check=False)
    assert failed.returncode == 2 and not failed.stdout and "disagrees" in failed.stderr
    assert before == snapshot()
    print(f'{counts["input"]} mapped, {counts["retained"]} retained. Changed keeper and altered span rejected.')
    for path in sorted(out.iterdir()):
        print(path.name, before[path.relative_to(directory)])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--baseline", type=Path, help="Earlier CLI to create the published export")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as tmp:
        exercise(Path(tmp), args.binary.resolve(), (args.baseline or args.binary).resolve())
