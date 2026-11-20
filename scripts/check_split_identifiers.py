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


def check_export(root, flags, originals, before, after, collision):
    def review(binary, *args, check=True):
        return subprocess.run([binary, "review-ids", *args, *flags], capture_output=True, check=check)

    sheet = root / "decisions.tsv"
    review(before, "--init-metadata-decisions", sheet)
    template = sheet.read_bytes()
    review(after, "--init-metadata-decisions", root / "new.tsv")
    assert (root / "new.tsv").read_bytes() == template
    # Inspect the actual colliding proceedings abstracts, including the distinguishing notes.
    comparison = review(after, "--inspect-pair", sheet, *collision).stdout.decode()
    for key in collision:
        assert f"[{key}] outside-decision-sheet (bytes " in comparison
    assert "course protocol" in comparison and "follow-up observations" in comparison
    for version, binary in [("old", before), ("new", after)]:
        review(binary, "--export-reviewed", sheet, root / version)
    for name in ("reviewed.ris", "mapping.tsv", "reconciliation.tsv"):
        assert (root / "old" / name).read_bytes() == (root / "new" / name).read_bytes()
    assert (root / "new/reviewed.ris").read_bytes() == b"".join(originals.values())
    assert sheet.read_bytes() == template and all(path.read_bytes() == data for path, data in originals.items())
    # An inspect-only pair cannot be promoted by adding a well-formed removal to the sheet.
    bad = root / "invalid.tsv"
    bad.write_bytes(template.removesuffix(b"end\n") + b"catalog:1\tkeep\n"
                    b"index:1\tverified-duplicate-of=catalog:1\tRead both records.\nend\n")
    destination = root / "invalid-export"
    result = review(after, "--export-reviewed", bad, destination, check=False)
    assert result.returncode and not result.stdout and not destination.exists()
    assert b"unexpected rows" in result.stderr


def exercise(root, before, after):
    flags, truth, originals = contrast(root)
    positives, old, new = compare("fresh contrast", flags, truth, before, after)
    assert {truth[a] for a, _ in (new - old) & positives} == {
        "pages", "article", "split-pages", "case-spacing", "reversed-ids"}
    collision, = (new - old) - positives
    assert {truth[k] for k in collision} == {"shared-heading", "shared-heading-other"}
    assert queue(after, flags) == queue(after, flags[3:] + flags[:3])
    check_export(root, flags, originals, before, after, collision)

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
            assert all(copies[a].study % 4 == 1 for a, _ in positives - new)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="complementary-ids-") as temp:
        exercise(Path(temp), args.before.resolve(), args.after.resolve())
