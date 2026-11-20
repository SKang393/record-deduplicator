"""Measure direct review leads against generated report identities, before any decisions."""
import argparse
import subprocess
import tempfile
from collections import defaultdict
from dataclasses import dataclass, replace
from itertools import combinations
from pathlib import Path

from check_synthetic_review import write_corpus


@dataclass(frozen=True)
class Citation:
    title: str
    author: str = "Hêtre, Éva"
    year: str = "2021"
    kind: str = "JOUR"
    extra: str = ""
    coauthor: str = ""

    def ris(self, aliases):
        title, author, year = ("T1", "A1", "Y1") if aliases else ("TI", "AU", "PY")
        fields = [(title, self.title), (author, self.author), (author, self.coauthor), (year, self.year)]
        body = "".join(f"{tag}  - {value}\n" for tag, value in fields if value)
        return f"TY  - {self.kind}\n{body}{self.extra}ER  -\n".encode()


def write_edges(directory):
    # The last column is report identity, independent of DOI and metadata agreement.
    cases = [
        (Citation("Étude of estuary transects"), {"title": "ÉTUDE  of estuary transects",
                                                  "author": "HÊTRE,  ÉVA", "year": "2021/03/12"}, True),
        (Citation("海岸コホート追跡", author="山田, 葵"), {}, True),
        (Citation("Optical depth: a pilot"), {"title": "Optical depth – a pilot"}, True),
        (Citation("Spring canopy measurements"), {"author": "Hêtre, É."}, True),
        (Citation("Wetland recovery in winter"), {"year": "2022"}, True),
        (Citation("Archive of tidal gates", year=""), {}, True),
        (Citation("Wind at paired stations", extra="DO  - 10.0000/paired\n"), {"extra": ""}, True),
        (Citation("Streambed reanalysis", extra="DO  - 10.0000/core\n"),
         {"extra": "UR  - https://doi.org/10.0000/CORE\n"}, True),
        (Citation("A shared methods heading", coauthor="Alder, P.", extra="JO  - Fable Methods\n"),
         {"coauthor": "Larch, M.", "extra": "JO  - Fable Data\n"}, False),
        (Citation("Estuary cohort protocol"), {"title": "Estuary cohort results"}, False),
        (Citation("Glacier survey: part I"), {"title": "Glacier survey: part II"}, False),
        (Citation("Physical properties of frost", author="Glint, Nia"), {"author": "Glint, Noa"}, False),
        (Citation("Annual shoreline survey"), {"kind": "CONF"}, False),
        (Citation("Paper on river ice", extra="DO  - 10.0000/copied\n"),
         {"title": "Correction: Paper on river ice"}, False),
        (Citation("Coastal survey", author=""), {}, False),
        (Citation("Notes on flow", extra="DO  - 10.0000/west\n"),
         {"extra": "DO  - 10.0000/east\n"}, False),
    ]
    truth, args = {}, []
    for side in ("left", "right"):
        data = bytearray()
        for n, (original, changes, same_report) in enumerate(cases, 1):
            rec = original if side == "left" else replace(original, **changes)
            truth[f"{side}:{n}"] = n, ("shared" if same_report else side)
            data.extend(rec.ris(aliases=side == "right"))
        path = directory / f"{side}.ris"
        path.write_bytes(data)
        args.append(("--ris-source", side, str(path)))
    return truth, args


def measure(binary, args, truth, missed):
    run = subprocess.run([binary, "review-ids", *(arg for source in args for arg in source)],
                         capture_output=True, check=True)
    families = defaultdict(list)
    for key, report in truth.items():
        families[report].append(key)

    def pairs(group):
        return set(combinations(sorted(group), 2))

    true_pairs = {pair for members in families.values() for pair in pairs(members)}
    leads = {"candidate": set(), "triage": set()}
    mixed = {"candidate": 0, "triage": 0}
    for kind, _, keys, _ in (line.split("\t") for line in run.stdout.decode().splitlines()):
        if kind in leads:
            members = keys.split(",")
            leads[kind].update(pairs(members))
            mixed[kind] += len({truth[key] for key in members}) > 1
    combined = leads["candidate"] | leads["triage"]
    hit = combined & true_pairs
    seen_copies = missed & {key for pair in hit for key in pair}
    print(f"true pairs surfaced {len(hit)}/{len(true_pairs)}, distinct-report pairs surfaced "
          f"{len(combined - true_pairs)}, mixed groups {mixed}")
    if missed:
        print(f"previously retained ID-less copies surfaced {len(seen_copies)}/{len(missed)}")
    return leads, true_pairs, seen_copies


def exercise(directory, binary, baseline):
    _, copies, _, _, _, args = write_corpus(directory)
    truth = {key: rec.report for key, rec in copies.items()}
    first, missed = {}, set()
    for key, rec in copies.items():
        if first.setdefault(rec.report, key) != key and not (rec.doi or rec.pmid):
            missed.add(key)
    print("Four-source fixture")
    leads, true_pairs, surfaced = measure(binary, args, truth, missed)
    assert len(missed) == 191
    assert len(leads["candidate"] & true_pairs) == 1603
    assert len(leads["candidate"] - true_pairs) == 243
    if baseline:
        assert not surfaced
    else:
        assert surfaced == missed
        added = leads["triage"] - leads["candidate"]
        assert len(added) == 318 and added <= true_pairs
    print("Focused title/author/year cases")
    truth, args = write_edges(directory)
    edge_leads, _, _ = measure(binary, args, truth, set())
    if baseline:
        assert not leads["triage"] and not edge_leads["triage"]
    else:
        assert edge_leads["triage"] == {(f"left:{n}", f"right:{n}") for n in (1, 2, 7, 9)}
    assert edge_leads["candidate"] == {(f"left:{n}", f"right:{n}") for n in (8, 14)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--baseline", action="store_true", help="Check an identifier-only binary")
    options = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="review-triage-") as tmp:
        exercise(Path(tmp), options.binary.resolve(), options.baseline)
