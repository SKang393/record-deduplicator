"""Offline what-if probes over generated RIS only. Neither alternative feeds the review queue."""
from collections import Counter, defaultdict
from itertools import combinations
from pathlib import Path
import re
import subprocess
from urllib.parse import urlsplit

from check_large_review import render
from check_synthetic_review import write_corpus
from check_title_variants import Record, direct_pairs, queue


def fresh(root):
    # Invent the source fields first. Neither the RIS writer nor the two probes
    # sees the separately adjudicated pair keys below.
    grain = ("Observers sorted repeated sand images by grain size before recording the median "
             "sorting error for each of four illumination levels in the coastal classroom.")
    light = ("Two groups repeated the light table exercise after exchanging seats and marked "
             "the smallest contour interval they could consistently distinguish on each slide.")
    shared_text = ("All stations contribute monthly contour readings using the same calibration "
                   "cards and the field staff document every instrument change in the common station register.")

    def tags(abstract="", article="", note=""):
        return (("AB  - " + abstract + "\n") if abstract else "") + (
            f"UR  - https://journal.example/articles/{article}\n" if article else "") + (
            "N1  - " + note + "\n" if note else "")

    # Title, pagination and additional fields for two independently sorted exports.
    rows = [
        ("Classifying sand images under changing illumination", "ahead of print", tags(grain, "2471"),
         "57-62", tags(grain, "2471")),
        ("Reading contour intervals on illuminated slides", "", tags(light), "", tags(light)),
        ("Hand tracing dune boundaries in winter", "", tags(article="7028"), "e17394", tags(article="7028")),
        ("Sorting elevation profiles from coastal trails", "", "", "", ""),
        ("Recording contour shifts at tidal stations", "", tags(shared_text, note="Station sampling protocol"),
         "", tags(shared_text, note="Separate report on observations after eighteen months")),
        ("Calibration of sensors along the tidal reach", "", tags(article="5013", note="Planned calibration protocol"),
         "", tags(article="5013", note="Follow-up observations. The series link points to the original protocol.")),
        ("Field sketches of a shifting inlet", "", "N1  - Volunteer protocol\n",
         "", "N1  - Return survey after the winter storms\n"),
        ("Rotating field images before estimating slope", "51-58", "", "51", "EP  - 58\n"),
        ("Counting landmarks across weekly shoreline transects", "", tags(article="8602") + "M3  - Protocol\n",
         "", tags(article="8602") + "M3  - Follow-up\n"),
    ]
    canonical, flags = {}, []
    for side, (label, order) in enumerate([("coast", range(1, 10)), ("ridge", (5, 9, 2, 8, 3, 1, 7, 4, 6))]):
        path = root / (label + ".ris")
        data = bytearray()
        for n, i in enumerate(order, 1):
            title, pages_a, extra_a, pages_b, extra_b = rows[i - 1]
            pages, extra = (pages_b, extra_b) if side else (pages_a, extra_a)
            # Alternate the namespace independently of identity and source label.
            identifier = (f"DO  - 10.0000/dune.{900 + i * 17}\n" if (side + i) % 2 else
                          f"UR  - https://pubmed.ncbi.nlm.nih.gov/{83000070 + i * 3}/\n")
            raw = Record(title, pages=pages, journal="Invented Coastal Cartography",
                         authors=("Willow, Ida", "Kite, Ren"), extra=extra + identifier).ris(bool(side))
            canonical[f"{label}:{n}"] = raw
            data.extend(raw)
        path.write_bytes(data)
        flags.extend(("--ris-source", label, path))
    truth = {("coast:1", "ridge:6"), ("coast:2", "ridge:3"), ("coast:3", "ridge:5"),
             ("coast:4", "ridge:8"), ("coast:8", "ridge:4")}
    return flags, canonical, truth


def fields_of(raw):
    # Both fixture writers supply canonical LF records; continuations belong to their tag.
    fields = defaultdict(list)
    for line in raw.decode().splitlines():
        if line.startswith("      "):
            fields[tag][-1] += " " + line.strip()
        else:
            tag = {"T1": "TI", "A1": "AU", "JF": "JO"}.get(line[:2], line[:2])
            fields[tag].append(line[6:].strip())
    return fields


def probe(rows, canonical):
    # Build two independent evidence indexes across the entire input. Require a
    # value to occur on exactly two records before considering any citation pair.
    abstracts, links = defaultdict(set), defaultdict(set)
    fields = {k: fields_of(raw) for k, raw in canonical.items()}
    citation = {}
    for k, f in fields.items():
        citation[k] = tuple(tuple(f[t]) for t in ("TI", "AU", "PY", "TY", "JO", "VL", "IS", "M3"))
        if len(f["AB"]) == 1:
            abstract = " ".join(f["AB"][0].split())
            if len(abstract.split()) >= 20:
                abstracts[abstract].add(k)
        # Literal non-resolver HTTPS publisher article path, including an item
        # number. Only this URL form appears as publisher evidence in the fixture.
        article_urls = {u for u in f["UR"] if re.fullmatch(
            r"https://[A-Za-z0-9.-]+/articles/[A-Za-z0-9_-]*[0-9][A-Za-z0-9_-]*", u)
            and urlsplit(u).hostname not in {"doi.org", "dx.doi.org", "pubmed.ncbi.nlm.nih.gov"}}
        if len(article_urls) == 1:
            links[article_urls.pop()].add(k)

    retained = {k for kind, _, k, info in (r.split("\t") for r in rows)
                if kind == "source" and info.endswith(";disposition=retained")}
    def namespaces(k):
        f = fields[k]
        return bool(f["DO"]), any(u.startswith("https://pubmed.ncbi.nlm.nih.gov/") for u in f["UR"])

    # All other citation requirements are positive and exact. Withhold crowded
    # citations and pairs already queued, even when their evidence value is rare.
    counts = Counter(citation.values())
    seen = direct_pairs(rows)
    def proposals(index):
        pairs = set()
        for keys in index.values():
            if len(keys) != 2 or not keys <= retained:
                continue
            a, b = sorted(keys)
            if (a.split(":")[0] == b.split(":")[0] or (a, b) in seen
                    or {namespaces(a), namespaces(b)} != {(True, False), (False, True)}
                    or citation[a] != citation[b] or counts[citation[a]] != 2
                    or not all(citation[a][:6]) or len(fields[a]["TI"][0].split()) < 5):
                continue
            if any(not fields[k]["SP"] or fields[k]["SP"] == ["ahead of print"] for k in (a, b)):
                pairs.add((a, b))
        return pairs
    return proposals(abstracts), proposals(links)


def score(label, flags, canonical, truth, cli):
    rows = queue(cli, flags)
    old = direct_pairs(rows)
    base_groups = sum(row.startswith(("candidate\t", "triage\t")) for row in rows)
    print(f"{label} baseline: {len(old & truth)}/{len(truth)} true pairs, {len(old - truth)} false, groups={base_groups}")
    alternatives = probe(rows, canonical)
    for name, added in zip(("distinctive abstract", "publisher article URL"), alternatives):
        print(f"{label} + {name}: {len(added & truth)} recovered, {len(added - truth)} extra false, "
              f"{base_groups+len(added)} total groups")
    return old, alternatives


def check_review(job, flags, canonical, before, after, inspections):
    job.mkdir()
    originals = {path: path.read_bytes() for path in map(Path, flags[2::3])}
    sheet = job / "choices.tsv"

    def review(binary, *args):
        return subprocess.check_output([binary, "review-ids", *args, *flags])

    initial = review(after, "--init-metadata-decisions", sheet)
    template = sheet.read_bytes()
    review(after, "--export-reviewed", sheet, job / "current")
    assert (job / "current/reviewed.ris").read_bytes() == b"".join(canonical.values())
    if before:
        old_sheet = job / "base.tsv"
        assert review(before, "--init-metadata-decisions", old_sheet) == initial
        assert old_sheet.read_bytes() == template
        assert review(before, "--pending-decisions", old_sheet) == review(after, "--pending-decisions", sheet)
        review(before, "--export-reviewed", old_sheet, job / "base")
        for name in ("reviewed.ris", "mapping.tsv", "reconciliation.tsv"):
            assert (job / "current" / name).read_bytes() == (job / "base" / name).read_bytes()

    for pair, expected in inspections:
        args = ("--inspect-pair", sheet, *pair)
        view = review(after, *args).decode().splitlines()
        notes = [line for line in view if line.startswith(tuple(key + " " for key in pair))]
        assert notes == expected, notes
        if before:
            assert [line for line in view if line not in notes] == review(before, *args).decode().splitlines()
    assert all(path.read_bytes() == data for path, data in originals.items())
    assert sheet.read_bytes() == template


def exercise(root, before, cli):
    flags, canonical, truth = fresh(root)
    old, (abstract, link) = score("fresh", flags, canonical, truth, cli)
    assert old == {("coast:8", "ridge:4")}
    assert abstract == {("coast:1", "ridge:6"), ("coast:2", "ridge:3"), ("coast:5", "ridge:1")}
    assert link == {("coast:1", "ridge:6"), ("coast:3", "ridge:5"), ("coast:6", "ridge:9")}

    absent = " has no SP or EP field for a final page range or article number."
    unusable = " supplies pagination fields without a usable final page range or article number."
    check_review(root / "fresh-review", flags, canonical, before, cli, [
        (("coast:1", "ridge:6"), ["coast:1" + unusable]),  # ahead-of-print vs final range
        (("coast:3", "ridge:5"), ["coast:3" + absent]),   # missing vs article number
        (("coast:8", "ridge:4"), []),                     # inline vs separate final pages
    ])

    _, copies, canonical, _, _, sources = write_corpus(root, studies=5400, render=render)
    flags = [arg for source in sources if source[1] in ("index", "update") for arg in source]
    canonical = {k: raw for k, raw in canonical.items() if k.split(":")[0] in flags[1::3]}
    reports = defaultdict(list)
    for k in canonical:
        reports[copies[k].report].append(k)
    truth = {p for keys in reports.values() for p in combinations(sorted(keys), 2)}
    old, alternatives = score("frozen", flags, canonical, truth, cli)
    missing = truth - old
    assert len(missing) == 164 and not any(alternatives)
    missed_fields = [fields_of(canonical[k]) for pair in missing for k in pair]
    assert all(not (f["SP"] or f["EP"]) for f in missed_fields)
    boilerplate, = {tuple(f["AB"]) for f in missed_fields}
    assert boilerplate and all(tuple(fields_of(raw)["AB"]) == boilerplate for raw in canonical.values())
    assert all(urlsplit(u).hostname == "pubmed.ncbi.nlm.nih.gov" for f in missed_fields for u in f["UR"])
    print(f"All {len(missed_fields)} missed occurrences lack SP and EP. They have no publisher article URL; "
          f"their abstract occurs in all {len(canonical)} records.")
    pair = min(missing)
    check_review(root / "frozen-review", flags, canonical, before, cli, [
        (pair, [key + absent for key in pair]),
    ])
