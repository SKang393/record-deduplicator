"""Direct candidate pairs for complementary identifiers. All records and labels are synthetic."""
import argparse
from collections import defaultdict
from dataclasses import replace
from itertools import combinations
from pathlib import Path
import subprocess
import tempfile

from check_large_review import planned_review, render
from check_synthetic_review import write_corpus
from check_title_variants import Record, challenge, direct_pairs, queue


def contrast(root):
    records = {"catalog": [], "index": []}

    def pair(name, same_report, title, *, left=None, right=None, left_ids=None, right_ids=None):
        n = len(records["catalog"]) + 1
        base = Record(title, authors=("Linden, Ana", "Gorse, Tim"), journal="Synthetic Atlas Quarterly")
        for label, family, fields, ids in (
            ("catalog", name, left, left_ids if left_ids is not None else f"DO  - 10.0000/atlas.{n}\n"),
            ("index", name if same_report else name + "-other", right,
             right_ids if right_ids is not None else f"UR  - https://pubmed.ncbi.nlm.nih.gov/{80000000+n}/\n"),
        ):
            record = replace(base, **(fields or {}))
            records[label].append((family, replace(record, extra=record.extra + ids)))

    pair("pages", True, "Reading contour lines under dim illumination")
    pair("article", True, "Finding steep slopes on classroom maps",
         left={"pages": "e77141"}, right={"pages": "e77141"})
    pair("case-spacing", True, "Tracing stream networks on classroom maps",
         right={"title": "TRACING stream networks   on classroom maps"})
    pair("split-pages", True, "Rotating maps during a navigation task",
         right={"pages": "131", "extra": "EP  - 138\n"})
    pair("reversed-ids", True, "Tracing routes through a model village",
         left_ids="UR  - https://pubmed.ncbi.nlm.nih.gov/81000001/\n", right_ids="DO  - 10.0000/route\n")
    pair("missing-pages", True, "Locating paths across several paper maps", right={"pages": ""})
    pair("year-drift", True, "Comparing route plans with field diaries", right={"year": "2020"})
    pair("missing-author", True, "Teaching map orientation with hand signals", right={"authors": ()})
    pair("protocol", False, "Judging distances using folded maps: protocol",
         right={"title": "Judging distances using folded maps: results"})
    pair("follow-up", False, "Recalling landmarks after a walking lesson",
         right={"title": "Recalling landmarks after a walking lesson: follow-up"})
    pair("different-pages", False, "Learning to read layered trail maps", right={"pages": "151-158"})
    pair("different-team", False, "Identifying paths on raised relief maps",
         right={"authors": ("Linden, Ana", "Elm, Lia")})
    pair("different-type", False, "Drawing paths between remembered field sites",
         right={"kind": "CPAPER"})
    pair("version", False, "Revisiting distances along a winding route",
         left={"extra": "M3  - Study protocol\n"}, right={"extra": "M3  - Follow-up\n"})
    pair("doi-conflict", False, "Describing landmarks along narrow country lanes",
         right_ids="UR  - https://pubmed.ncbi.nlm.nih.gov/82000001/\nDO  - 10.0000/other\n")
    pair("pmid-conflict", False, "Using grid references to locate landmarks",
         left_ids="DO  - 10.0000/grid\nUR  - https://pubmed.ncbi.nlm.nih.gov/82000002/\n")
    pair("internal-conflict", False, "Guessing distance from shaded relief maps",
         left_ids="DO  - 10.0000/a\nDO  - 10.0000/b\n")
    pair("shared-heading", False, "Learning to interpret shaded route maps",
         left={"extra": "N1  - Proceedings abstract: course protocol\n"},
         right={"extra": "N1  - Separate proceedings abstract: follow-up observations\n"})
    pair("crowded", False, "Navigating crossings by reading street maps")
    records["catalog"].append(("crowded-third", replace(records["catalog"][-1][1], extra="DO  - 10.0000/third\n")))
    pair("shared-id", True, "Plotting travel time between familiar landmarks",
         left_ids="DO  - 10.0000/shared\n", right_ids="DO  - 10.0000/shared\n")

    truth, flags, originals = {}, [], {}
    for label, rows in records.items():
        path = root / (label + ".ris")
        buf = bytearray()
        for n, (report, record) in enumerate(rows, 1):
            truth[f"{label}:{n}"] = report
            buf.extend(record.ris(label == "index"))
        originals[path] = bytes(buf)
        path.write_bytes(buf)
        flags.extend(("--ris-source", label, path))
    return flags, truth, originals


SIGNAL = "triage\tcomplementary_id_locator\t"


def compare(label, flags, truth, before, after):
    families = defaultdict(list)
    for key, report in truth.items():
        families[report].append(key)
    positives = {p for members in families.values() for p in combinations(sorted(members), 2)}
    baseline, current = queue(before, flags), queue(after, flags)
    added = [line for line in current if line.startswith(SIGNAL)]
    assert all(line.endswith("\tinspect_only") and len(line.split("\t")[2].split(",")) == 2 for line in added)
    assert [line for line in current if not line.startswith(SIGNAL)] == baseline
    old_pairs, new_pairs = direct_pairs(baseline), direct_pairs(current)
    assert len(new_pairs - old_pairs) == len(added)
    for version, seen in [("before", old_pairs), ("after", new_pairs)]:
        print(f"{label} {version}: {len(seen & positives)}/{len(positives)} true pairs, "
              f"{len(seen - positives)} distinct, {len(seen)} to inspect")
    return positives, old_pairs, new_pairs


def check_undecided(root, flags, before, after):
    root.mkdir()
    sheet = root / "decisions.tsv"
    answers = [str(sheet)]
    for label, path in zip(flags[1::3], flags[2::3]):
        answers.extend((label, str(path)))
    guided = subprocess.run([after, "start"], input="\n".join([*answers, "", "yes", ""]).encode(),
                            capture_output=True, check=True)
    groups = [line for line in guided.stdout.decode().splitlines() if line.startswith(("candidate\t", "triage\t"))]
    added = [line for line in groups if line.startswith(SIGNAL)]
    assert added and groups[-len(added):] == added
    assert all(line.endswith("\tinspect_only") for line in added)
    assert f"optional comparisons ({len(added)} total)" in guided.stderr.decode()
    pending = subprocess.check_output([after, "review-ids", "--pending-decisions", sheet, *flags])
    subprocess.run([after, "review-ids", "--export-reviewed", sheet, root / "new", *flags], check=True)
    if before:
        old_sheet = root / "baseline.tsv"
        initial = subprocess.check_output([before, "review-ids", "--init-metadata-decisions", old_sheet, *flags])
        assert [r for r in guided.stdout.decode().splitlines() if not r.startswith(SIGNAL)] == initial.decode().splitlines()
        assert old_sheet.read_bytes() == sheet.read_bytes()
        assert subprocess.check_output([before, "review-ids", "--pending-decisions", old_sheet, *flags]) == pending
        subprocess.run([before, "review-ids", "--export-reviewed", old_sheet, root / "old", *flags], check=True)
        for name in ("reviewed.ris", "mapping.tsv", "reconciliation.tsv"):
            assert (root / "old" / name).read_bytes() == (root / "new" / name).read_bytes()
    summary, = [line for line in pending.decode().splitlines() if line.startswith("summary\tpending\t")]
    print(f"Full queue: {len(groups) - len(added)} prior groups followed by {len(added)} optional pairs. {summary}")
    return sheet, added


def check_contrast(root, before, after):
    flags, truth, originals = contrast(root)
    job = root / "contrast-review"
    sheet, added = check_undecided(job, flags, before, after)
    template = sheet.read_bytes()
    pairs = direct_pairs(added)
    assert len(added) == len(pairs) == 6
    assert {truth[a] for a, b in pairs if truth[a] == truth[b]} == {
        "pages", "article", "split-pages", "case-spacing", "reversed-ids"}
    collision, = [(a, b) for a, b in pairs if truth[a] != truth[b]]
    assert {truth[k] for k in collision} == {"shared-heading", "shared-heading-other"}
    if before:
        compare("fresh contrast", flags, truth, before, after)
    assert queue(after, flags) == queue(after, flags[3:] + flags[:3])
    comparison = subprocess.check_output([after, "review-ids", "--inspect-pair", sheet, *collision, *flags]).decode()
    for key in collision:
        assert f"[{key}] outside-decision-sheet (bytes " in comparison
    assert "course protocol" in comparison and "follow-up observations" in comparison
    assert (job / "new/reviewed.ris").read_bytes() == b"".join(originals.values())
    mapped = [line.split("\t") for line in (job / "new/mapping.tsv").read_text(encoding="utf-8").splitlines()]
    outputs = {detail.split(";output=")[1] for kind, status, key, detail in mapped
               if kind == "map" and status == "not-candidate" and key in collision}
    assert len(outputs) == 2
    left, right = collision
    extra = f"{left}\tkeep\n{right}\tverified-duplicate-of={left}\tCompared reports.\nend\n"
    bad = job / "invalid.tsv"
    bad.write_bytes(template.removesuffix(b"end\n") + extra.encode())
    destination = job / "invalid-export"
    result = subprocess.run([after, "review-ids", "--export-reviewed", bad, destination, *flags], capture_output=True)
    assert result.returncode and not result.stdout and not destination.exists()
    assert b"unexpected rows" in result.stderr
    assert sheet.read_bytes() == template and all(path.read_bytes() == data for path, data in originals.items())
    print(f"Inspected {left} and {right}: protocol and follow-up retained separately; attempted removal rejected.")


def exercise(root, before, after):
    check_contrast(root, before, after)
    if before is None:
        return

    flags, truth, _ = challenge(root)
    positives, old, new = compare("frozen title ledger", flags, truth, before, after)
    assert new == old
    print("Still missed:", ", ".join(sorted(truth[a] for a, _ in positives - new)))

    _, copies, _, _, _, sources = write_corpus(root, studies=5400, render=render)
    for groups in [sources, [s for s in sources if s[1] in ("index", "update")]]:
        flags = [arg for source in groups for arg in source]
        labels = flags[1::3]
        truth = {k: r.report for k, r in copies.items() if k.split(":")[0] in labels}
        positives, old, new = compare("+".join(labels), flags, truth, before, after)
        missed = positives - old
        assert len(missed) == 654
        for a, b in missed:
            assert a.startswith("index:") and copies[a].pmid and not copies[a].doi
            assert b.startswith("update:") and copies[b].doi and not copies[b].pmid
        endpoints = {key for p in old & positives for key in p}
        if len(groups) == len(sources):
            _, _, deferred = planned_review(copies)
            targets = {k for k, reason in deferred.items() if reason == "variant title without a shared ID"}
            assert len(targets) == 111 and targets <= endpoints
            assert {key for p in positives for key in p} == endpoints and new == old
            neighbors = defaultdict(set)
            for a, b in old & positives:
                neighbors[a].add(b)
                neighbors[b].add(a)
            assert all(neighbors[a] & neighbors[b] for a, b in missed)
        else:
            assert all(a not in endpoints and b not in endpoints for a, b in missed)
            assert len(new - old) == 490 and new - old <= missed
            check_undecided(root / "slice-review", flags, before, after)
            assert all(copies[a].study % 4 == 1 for a, _ in positives - new)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path, nargs="?", help="Baseline CLI for frozen-corpus and export comparison")
    parser.add_argument("after", type=Path)
    parser.add_argument("--missing-pages", action="store_true", help="Probe independent evidence for the 164 misses")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="complementary-ids-") as temp:
        before = args.before.resolve() if args.before else None
        if args.missing_pages:
            from check_missing_pages import exercise as probe_missing_pages
            probe_missing_pages(Path(temp), before, args.after.resolve())
        else:
            exercise(Path(temp), before, args.after.resolve())
