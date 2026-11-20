"""Measure a simulated pre-screen duplicate review on four generated RIS exports."""
import argparse
import csv
import hashlib
import json
import subprocess
import tempfile
from collections import Counter, defaultdict
from itertools import combinations
from pathlib import Path
from time import perf_counter

from check_synthetic_review import serialize, write_corpus


def title_variant(rec, label):
    return label == "regional" and not (rec.doi or rec.pmid) and rec.study % 5 == 0


def render(rec, label, ordinal):
    data = serialize(rec, label, ordinal)
    topics = ("Marsh recovery", "Tidal mixing", "River ice", "Estuary sediments")
    authors = ("Alder, I.", "Reed, É.", "Moss, T.", "Oro, F.", "Elm, C.")
    journals = ("Fable Ecology", "Synthetic Water Reports", "Fable Estuaries")
    title = f"{topics[rec.study % len(topics)]} at site {rec.study}: {rec.kind}"
    if title_variant(rec, label):
        title += " (regional series)"
    data = data.replace(f"Cohort {rec.study}: {rec.kind}".encode(), title.encode())
    data = data.replace("Exemple, É.".encode(), authors[rec.study % len(authors)].encode())
    start = 10 * (rec.study % 50 + 1)
    journal = "JF" if label == "index" else "JO"
    locator = f"{journal}  - {journals[rec.study % len(journals)]}\nVL  - {rec.study % 12 + 1}\n"
    if rec.study % 4 != 1:  # Some reports have no final pages in any export.
        locator += f"SP  - {start}\nEP  - {start+6}\n" if label == "index" else f"SP  - {start}-{start+6}\n"
    return data.removesuffix(b"ER  -\n") + locator.encode() + b"ER  -\n"


def planned_review(copies):
    # Report identity and same-study/different-report labels already exist in
    # the generator. Never derive truth or choose a representative from a queue.
    first, edits, deferred = {}, {}, {}
    for key, rec in copies.items():
        target = first.setdefault(rec.report, key)
        if target == key:
            continue
        original = copies[target]
        shared = (rec.doi and rec.doi == original.doi) or (rec.pmid and rec.pmid == original.pmid)
        if rec.study % 13 == 0:
            deferred[key] = "deliberately deferred"
        elif shared:
            edits[key] = f"duplicate-of={target}"
        elif title_variant(rec, key.split(":")[0]):
            deferred[key] = "variant title without a shared ID"
        elif rec.study % 4 == 1:
            deferred[key] = "missing final pages without a shared ID"
        else:
            edits[key] = f"verified-duplicate-of={target}\tCompared authors, journal, volume and final pages."
        if key in edits:
            edits[target] = "keep"
    return first, edits, deferred


def exercise(binary, directory, baseline):
    sources, copies, canonical, original, spans, args = write_corpus(directory, studies=5400, render=render)
    first, edits, deferred = planned_review(copies)
    times = {}

    def run(stage, *flags):
        command = [binary, "review-ids", *flags, *(arg for group in args for arg in group)]
        start = perf_counter()
        result = subprocess.run(command, cwd=directory, capture_output=True, check=True)
        times[stage] = round(perf_counter() - start, 3)
        return result.stdout.decode().splitlines()

    sheet = directory / "choices.tsv"
    queue = run("import_and_init", "--init-metadata-decisions", sheet)
    groups, mixed, membership = Counter(), Counter(), defaultdict(set)
    pairs = set()
    for row in queue:
        kind, signal, members, _ = row.split("\t")
        if kind == "issue":
            membership["issue"].add(members)
        elif kind in {"candidate", "triage"}:
            keys = members.split(",")
            groups[signal] += 1
            mixed[signal] += len({copies[k].report for k in keys}) > 1
            membership[kind].update(keys)
            pairs.update(combinations(sorted(keys), 2))

    # The untouched sheet must export every occurrence, even in mixed ID groups.
    run("all_pending_export", "--export-reviewed", sheet, "pending")
    assert (directory / "pending/reviewed.ris").read_bytes() == b"".join(canonical.values())
    lines = sheet.read_bytes().decode().splitlines()
    editable = {line.split("\t")[0] for line in lines if line.endswith("\tpending")}
    id_counts = {field: Counter(getattr(rec, field) for rec in copies.values() if getattr(rec, field))
                 for field in ("doi", "pmid")}
    assert membership["candidate"] == {k for k, rec in copies.items()
                                       if id_counts["doi"][rec.doi] > 1 or id_counts["pmid"][rec.pmid] > 1}
    assert editable == membership["candidate"] | membership["triage"]
    assert edits.keys() <= editable
    for i, line in enumerate(lines):
        key, _, value = line.partition("\t")
        if value == "pending" and key in edits:
            lines[i] = f"{key}\t{edits[key]}"
    saved = ("\n".join(lines) + "\n").encode()
    sheet.write_bytes(saved)
    preview = run("resume", "--resume-decisions", sheet)
    assert sum(line.startswith("decision\t") for line in preview) == len(editable)
    pending_metrics = {}
    if not baseline:
        worklist = run("pending_view", "--pending-decisions", sheet)
        open_groups = [line.split("\t") for line in worklist if line.startswith("pending\t")]
        expected = editable - edits.keys()
        assert {key for _, _, members, _ in open_groups for key in members.split(",")} == expected
        assert worklist[1].startswith(f"summary\tpending\t{len(expected)}\tgroups={len(open_groups)};")
        assert [r for r in worklist if r.startswith("input\t")] == [r for r in queue if r.startswith("input\t")]
        pending_metrics = {"pending_view_lines": len(worklist), "pending_groups": len(open_groups),
                           "pending_occurrences": len(expected)}
    run("reviewed_export", "--export-reviewed", sheet, "reviewed")
    retained = [key for key in copies if not edits.get(key, "").startswith(("duplicate-of=", "verified-duplicate-of="))]
    ordinals = {key: n for n, key in enumerate(retained, 1)}
    out = directory / "reviewed"
    ris = (out / "reviewed.ris").read_bytes()
    assert ris == b"".join(canonical[key] for key in retained)
    kept_reports = {copies[key].report for key in retained}
    assert kept_reports == first.keys()
    assert set(retained) - set(first.values()) == deferred.keys()

    mapping = (out / "mapping.tsv").read_bytes()
    rows = [line.split("\t") for line in mapping.decode().splitlines()]
    mapped = [row for row in rows if row[0] == "map"]
    assert [row[2] for row in mapped] == list(copies)
    by_source = {label: Counter(input=len(records)) for label, records in sources.items()}
    for _, action, key, detail in mapped:
        expected = edits.get(key, "pending" if key in editable else "not-candidate").split("\t")[0]
        assert action == expected
        target = first[copies[key].report] if action.startswith(("duplicate-of=", "verified-duplicate-of=")) else key
        start, end = spans[key]
        assert detail == f"bytes={start}..{end};output={ordinals[target]}"
        counts = by_source[key.partition(":")[0]]
        if key == target:
            counts["retained"] += 1
        if action == "not-candidate":
            category = "outside_sheet_flagged" if key in membership["issue"] else "outside_sheet_unflagged"
        elif action == "pending":
            category = "pending_id" if key in membership["candidate"] else "pending_metadata"
        elif key != target:
            category = "removed_metadata" if action.startswith("verified-") else "removed_id"
        else:
            category = "keep"
        counts[category] += 1
    for label, data in original.items():
        assert (directory / f"{label}.ris").read_bytes() == data
        assert ["input", "ris", label, f"bytes={len(data)};occurrences={len(sources[label])};sha256={hashlib.sha256(data).hexdigest()}"] in rows
    assert sheet.read_bytes() == saved
    assert ["summary", "all", str(len(copies)), f"retained={len(retained)};suppressed={len(copies)-len(retained)}"] in rows
    assert ["output", "ris", "reviewed.ris", f"bytes={len(ris)};sha256={hashlib.sha256(ris).hexdigest()}"] in rows
    total = sum(by_source.values(), Counter())
    if not baseline:
        with (out / "reconciliation.tsv").open(encoding="utf-8", newline="") as stream:
            summary = list(csv.DictReader(stream, delimiter="\t"))
        assert [(r["scope"], r["source"]) for r in summary] == [("source", s) for s in sources] + [("total", "")]
        for row in summary:
            expected = by_source[row["source"]] if row["scope"] == "source" else total
            values = {k: int(v) for k, v in row.items() if k not in ("scope", "source")}
            assert values.keys() == total.keys()
            assert all(v == expected[k] for k, v in values.items())
    family_sizes = Counter(r.report for r in copies.values())
    print(json.dumps({
        "input_occurrences": {s: len(records) for s, records in sources.items()},
        "input_bytes": sum(len(b) for b in original.values()), "studies": 5400,
        "reports": dict(Counter(kind for _, kind in first)), "groups": groups, "mixed_report_groups": mixed,
        "true_pairs_in_fixture": sum(n * (n - 1) // 2 for n in family_sizes.values()),
        "distinct_direct_pairs": len(pairs), "distinct_report_pairs": sum(copies[a].report != copies[b].report for a, b in pairs),
        "editable_rows": len(editable), "resume_lines": len(preview), **pending_metrics, "map_rows": len(mapped),
        "dispositions_by_source": by_source, "total": total, "true_extra_copies_retained": Counter(deferred.values()),
        "cli_wall_seconds": times,
        "sha256": {"reviewed.ris": hashlib.sha256(ris).hexdigest(), "mapping.tsv": hashlib.sha256(mapping).hexdigest()},
    }, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--baseline", action="store_true", help="Measure the earlier binary without reconciliation.tsv")
    options = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="large-ris-review-") as tmp:
        exercise(options.binary.resolve(), Path(tmp), options.baseline)
