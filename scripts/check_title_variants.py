"""Measure direct review pairs in two independently labeled, generated RIS exports."""
import argparse
import json
import subprocess
import tempfile
from dataclasses import dataclass, replace
from itertools import combinations
from pathlib import Path


@dataclass(frozen=True)
class Record:
    title: str
    authors: tuple = ("Lune, Mira", "Rook, Pascale")
    year: str = "2019"
    pages: str = "131-138"
    kind: str = "JOUR"
    journal: str = "Fictional Journal of Applied Sound"
    extra: str = ""

    def ris(self, alias):
        title, author, journal = ("T1", "A1", "JF") if alias else ("TI", "AU", "JO")
        fields = [("TY", self.kind), (title, self.title), *((author, a) for a in self.authors),
                  ("PY", self.year), (journal, self.journal), ("VL", "12"), ("SP", self.pages)]
        return ("".join(f"{tag}  - {v}\n" for tag, v in fields if v) + self.extra + "ER  -\n").encode()


def challenge(root):
    # Labels describe the invented publications, never appear in RIS, and are
    # assigned before running a binary. All true copies lack a shared identifier.
    sources = {"atlas": [], "folio": []}

    def pair(name, same_report, original, **changes):
        sources["atlas"].append((name, original))
        sources["folio"].append((name if same_report else name + "-companion", replace(original, **changes)))

    pair("exact-control", True, Record("Mapping courtyard echoes at night"), title="MAPPING  courtyard echoes at night")
    pair("punctuation", True, Record("Porous tiles in rehearsal rooms: a comparison"),
         title="Porous tiles in rehearsal rooms — a comparison")
    pair("hyphen-initial", True, Record("Low-frequency noise along the towpath"),
         title="Low frequency noise along the towpath", authors=("Lune, M.",))
    pair("omitted-subtitle", True, Record("Listening fatigue during quartet rehearsals: a diary study"),
         title="Listening fatigue during quartet rehearsals", authors=())
    pair("parenthetical", True, Record("Tracing vibration through wooden gallery floors"),
         title="Tracing vibration through wooden gallery floors (technical series)")
    pair("dash-subtitle", True, Record("Resonance of suspended clay panels — a field survey"),
         title="Resonance of suspended clay panels")
    pair("article-number", True, Record("Rainfall sounds on copper canopies", pages="e43821"),
         title="Rainfall sounds on copper canopies: paired measurements", authors=("Lune, M.",))
    pair("omitted-coauthor", True, Record("Quiet intervals, pupil attention and recall"),
         title="Quiet intervals: pupil attention and recall", authors=("Lune, Mira",))
    pair("spelling", True, Record("Fibre damping in thin practice screens"), title="Fiber damping in thin practice screens")
    pair("plain-truncation", True, Record("Auditory load during prolonged studio practice"),
         title="Auditory load during prolonged")
    pair("year-drift", True, Record("Echo patterns in ceramic galleries"),
         title="Echo patterns in ceramic galleries: a survey", year="2020")
    pair("missing-pages", True, Record("Rain noise in covered courtyards"), title="Rain noise in covered courtyards (survey)", pages="")
    pair("missing-author", True, Record("How singers localize distant bells"), authors=())
    pair("exact-title-distinct", False, Record("Sound absorption in woven mats"), pages="151-158")
    pair("numbered-parts", False, Record("Measuring vibration in timber shells: part I"),
         title="Measuring vibration in timber shells: part II")
    pair("protocol-results", False, Record("Hearing in open plan workshops: protocol"),
         title="Hearing in open plan workshops: results")
    # Two short proceedings contributions share a page range. One title is
    # the other's main title; the shared coordinates alone cannot identify a report.
    pair("same-pages", False, Record("Listener comfort beneath glass roofs"),
         title="Listener comfort beneath glass roofs: evening sessions", authors=("Reeve, Jo",))
    pair("same-team-supplement", False, Record("Mapping resonances along narrow stairwells", pages="S1-S4"),
         title="Mapping resonances along narrow stairwells: afternoon sessions", authors=("Lune, M.",))
    pair("different-pages", False, Record("Footfall spectra in rehearsal spaces"),
         title="Footfall spectra in rehearsal spaces: winter sessions", pages="231-239")
    pair("different-type", False, Record("Porous liners in station halls"),
         title="Porous liners in station halls: a pilot", kind="CPAPER")
    pair("contradictory-ids", False, Record("Tuning curves for reed whistles", extra="DO  - 10.0000/reed-a\nDO  - 10.0000/reed-b\n"),
         title="Tuning curves for reed whistles (pilot series)", extra="")
    # A crowded supplement has several short reports at the same location.
    crowded = Record("Sound reflections in model theatres", pages="S8-S9")
    pair("crowded", True, crowded, title=crowded.title + ": scale tests")
    for source, suffix in [("atlas", "field notes"), ("folio", "appendix"), ("atlas", "follow-up")]:
        sources[source].append((suffix, replace(crowded, title=crowded.title + ": " + suffix)))

    flags, truth, originals = [], {}, {}
    for label, records in sources.items():
        if label == "folio":
            records.reverse()
        path = root / (label + ".ris")
        data = bytearray()
        for ordinal, (report, record) in enumerate(records, 1):
            truth[f"{label}:{ordinal}"] = report
            data.extend(record.ris(label == "folio"))
        originals[path] = bytes(data)
        path.write_bytes(data)
        flags.extend(("--ris-source", label, path))
    return flags, truth, originals


def direct_pairs(lines):
    seen = set()
    for line in lines:
        kind, signal, members, _ = line.split("\t")
        if kind in ("candidate", "triage"):
            keys = sorted(members.split(","))
            if signal == "title_variant_locator":
                assert len(keys) == 2
            seen.update(combinations(keys, 2))
    return seen


def queue(binary, flags):
    return subprocess.check_output([binary, "review-ids", *flags]).decode().splitlines()


def score(label, lines, truth):
    seen = direct_pairs(lines)
    positives = {(a, b) for a, b in combinations(sorted(truth), 2) if truth[a] == truth[b]}
    print(json.dumps({
        "challenge": label, "occurrences": len(truth), "true_pairs": len(positives), "review_pairs": len(seen),
        "true_surfaced": len(positives & seen), "distinct_surfaced": len(seen - positives),
        "missed": sorted(truth[a] for a, _ in positives - seen),
        "distinct_pairs": sorted(sorted([truth[a], truth[b]]) for a, b in seen - positives),
    }))
    return {truth[a] for a, _ in positives & seen}, seen - positives


def check_review(root, binary, before):
    flags, truth, originals = challenge(root)
    lines = queue(binary, flags)
    hits, false_pairs = score("revised", lines, truth)
    assert hits == {"exact-control", "punctuation", "hyphen-initial", "omitted-subtitle", "parenthetical",
                    "dash-subtitle", "article-number", "omitted-coauthor"}
    assert {frozenset((truth[a], truth[b])) for a, b in false_pairs} == {
        frozenset((name, name + "-companion")) for name in ("exact-title-distinct", "same-team-supplement")}
    signal = "triage\ttitle_variant_locator\t"
    leads = [line for line in lines if line.startswith(signal)]
    assert len(leads) == len(direct_pairs(leads)) == 8
    assert lines == queue(binary, flags[3:] + flags[:3])
    assert any("conflicting_dois_in_record" in line for line in lines)
    sheet = root / "choices.tsv"

    def review(cli, *args, check=True):
        return subprocess.run([cli, "review-ids", *args, *flags], check=check, capture_output=True)

    review(binary, "--init-metadata-decisions", sheet)
    template = sheet.read_bytes()
    if before:
        old = queue(before, flags)
        old_hits, old_false = score("original", old, truth)
        assert old_hits == {"exact-control"} and len(old_false) == 1
        assert [line for line in lines if not line.startswith(signal)] == old
        old_sheet = root / "original.tsv"
        review(before, "--init-metadata-decisions", old_sheet)
        assert old_sheet.read_bytes() == template

    # All suggestions are inspectable, even outside the sheet. No new
    # eligibility, and no suppressed occurrences in an unedited export.
    def keys(report):
        return sorted(key for key, family in truth.items() if family == report)

    left, right = keys("omitted-subtitle")
    inspected = review(binary, "--inspect-pair", sheet, left, right).stdout.decode()
    for key in (left, right):
        assert f"[{key}] not-candidate" in inspected
    pending = root / "pending"
    review(binary, "--export-reviewed", sheet, pending)
    assert (pending / "reviewed.ris").read_bytes() == b"".join(originals.values())
    assert sheet.read_bytes() == template
    # Try otherwise well-formed edits, with the extra rows in sheet order.
    lines = template.decode().splitlines()
    header = [line for line in lines if line != "end" and line.split("\t", 1)[0] not in truth]
    choices = dict(line.split("\t", 1) for line in lines if line.split("\t", 1)[0] in truth)
    choices.update({left: "keep", right: f"verified-duplicate-of={left}\tChecked the report."})
    edits = [f"{key}\t{choices[key]}" for key in truth if key in choices]
    sheet.write_bytes(("\n".join([*header, *edits, "end"]) + "\n").encode())
    invalid = root / "invalid"
    result = review(binary, "--export-reviewed", sheet, invalid, check=False)
    assert result.returncode and not result.stdout and not invalid.exists()

    left, right = keys("exact-control")
    approved = template.replace(f"{left}\tpending".encode(), f"{left}\tkeep".encode()).replace(
        f"{right}\tpending".encode(), f"{right}\tverified-duplicate-of={left}\tCompared authors and final pages.".encode())
    sheet.write_bytes(approved)
    reviewed = root / "reviewed"
    review(binary, "--export-reviewed", sheet, reviewed)
    assert b"retained=46;suppressed=1" in (reviewed / "mapping.tsv").read_bytes()
    if before:
        prior = root / "prior"
        review(before, "--export-reviewed", sheet, prior)
        for name in ("reviewed.ris", "mapping.tsv", "reconciliation.tsv"):
            assert (reviewed / name).read_bytes() == (prior / name).read_bytes()
    assert all(path.read_bytes() == data for path, data in originals.items())
    assert sheet.read_bytes() == approved
    print("PASS: inspect-only leads, source bytes, sheet eligibility, guarded export and existing verified decision.")


def scale_targets(root, binary, label):
    # Historical development corpus, scored separately from the new challenge.
    from check_large_review import planned_review, render
    from check_synthetic_review import write_corpus

    _, copies, _, _, _, sources = write_corpus(root, studies=5400, render=render)
    _, _, deferred = planned_review(copies)
    targets = {key for key, reason in deferred.items() if reason == "variant title without a shared ID"}
    assert len(targets) == 111
    pairs = direct_pairs(queue(binary, [arg for source in sources for arg in source]))
    true = {(a, b) for a, b in pairs if copies[a].report == copies[b].report}
    surfaced = targets & {key for pair in true for key in pair}
    print(json.dumps({"development_corpus": label, "target_copies": len(targets),
                      "target_copies_with_true_lead": len(surfaced), "review_pairs": len(pairs),
                      "distinct_surfaced": len(pairs - true)}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--before", type=Path, help="Compare with the original CLI on the same inputs")
    parser.add_argument("--scale-targets", action="store_true", help="Also score the earlier 111 target copies separately")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="title-variants-") as temp:
        root = Path(temp)
        binary = args.binary.resolve()
        before = args.before.resolve() if args.before else None
        check_review(root, binary, before)
        if args.scale_targets:
            for label, cli in (("original", before), ("revised", binary)):
                if cli:
                    scale_targets(root, cli, label)
