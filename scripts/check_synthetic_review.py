"""Exercise the real CLI with generated RIS and a separate, constructed report truth table."""
import argparse
import hashlib
import json
import subprocess
import tempfile
from collections import Counter
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Copy:
    study: int
    kind: str
    doi: str
    pmid: str

    @property
    def report(self):
        return self.study, self.kind


def corpus():
    sources = {label: [] for label in ("central", "index", "regional", "update")}

    def add(label, study, kind):
        report_number = 3 * study + ("results", "protocol", "follow-up").index(kind)
        # Some single-copy protocols carry the results DOI but have their own PMID.
        doi_number = 3 * study if kind == "protocol" and study % 8 == 0 else report_number
        doi, pmid = f"10.0000/syn.{doi_number}", str(9000000000 + report_number)
        if kind == "results" and (study % 11 == 0 or (label == "regional" and study % 7 == 0)):
            doi = pmid = ""
        elif label == "index":
            doi = ""
        elif label == "update":
            pmid = ""
        sources[label].append(Copy(study, kind, doi, pmid))

    for study in range(1, 1001):
        add("central", study, "results")
        if study % 3:
            add("index", study, "results")
        if study % 2 == 0:
            add("regional", study, "results")
        if study % 5 == 0:
            add("update", study, "results")
        if study % 30 == 0:
            add("update", study, "results")  # overlapping pages within one export
        if study % 4 == 0:
            add("regional", study, "protocol")
        if study % 6 == 0:
            add("index", study, "follow-up")
    sources["regional"].reverse()  # independent database sort order
    return sources


def serialize(copy, label, ordinal):
    title_tag = "T1" if label == "regional" else "TI"
    lines = ["TY  - JOUR", f"AN  - LOCAL-{ordinal}", f"{title_tag}  - Cohort {copy.study}: {copy.kind}",
             "AU  - Exemple, É.", "AU  - Sample, R.", f"PY  - {2000 + copy.study % 25}",
             f"N1  - Study registry SYN-{copy.study}", "AB  - Síntesis de una cohorte.",
             "      Résumé • 方法", "KW  - cohort", "KW  - follow-up"]
    if copy.doi:
        lines.append(f"UR  - https://doi.org/{copy.doi.upper()}" if label == "regional"
                     else f"DO  - {copy.doi}")
    if copy.pmid:
        lines.append(f"UR  - https://pubmed.ncbi.nlm.nih.gov/{copy.pmid}/")
    lines.append("ER  -")
    return ("\n".join(lines) + "\n").encode()


def write_corpus(directory):
    sources = corpus()
    copies, canonical, original, spans = {}, {}, {}, {}
    source_args = []
    for label, records in sources.items():
        path = directory / f"{label}.ris"
        data = bytearray()
        for i, rec in enumerate(records, 1):
            key = f"{label}:{i}"
            copies[key] = rec
            canonical[key] = serialize(rec, label, i)
            eol = {"central": b"\r\n", "index": b"\n", "regional": b"\r",
                   "update": (b"\r\n", b"\n", b"\r")[i % 3]}[label]
            raw = canonical[key].replace(b"\n", eol)
            if i == 1 and label in ("central", "regional"):
                raw = b"\xef\xbb\xbf" + raw
            if i == len(records):
                raw = raw.removesuffix(eol)
            spans[key] = (len(data), len(data) + len(raw))
            data.extend(raw)
            if i % 20 == 0 and i != len(records):
                data.extend(eol)
        original[label] = bytes(data)
        path.write_bytes(data)
        source_args.append(("--ris-source", label, str(path)))
    return sources, copies, canonical, original, spans, source_args


def exercise(directory, binary):
    sources, copies, canonical, original, spans, source_args = write_corpus(directory)
    # Set expected decisions before invoking the engine. Identity comes only from
    # the study/report table. The shared-ID counts model which rows can be edited.
    counts = [Counter(getattr(r, field) for r in copies.values() if getattr(r, field))
              for field in ("doi", "pmid")]
    first, choice, disposition = {}, {}, {}
    for key, rec in copies.items():
        representative = first.setdefault(rec.report, key)
        candidate = counts[0][rec.doi] > 1 or counts[1][rec.pmid] > 1
        if not candidate:
            disposition[key] = "retained" if rec.doi or rec.pmid else "review_input"
            choice[key] = "not-candidate"
        else:
            disposition[key] = "review_candidate"
            if representative == key:
                choice[key] = "keep"
            elif rec.study % 13 == 0:
                choice[key] = "pending"  # deliberately defer a known duplicate
            else:
                choice[key] = f"duplicate-of={representative}"

    def run(*args, reverse=False, check=True):
        groups = reversed(source_args) if reverse else source_args
        return subprocess.run([binary, "review-ids", *args, *(arg for group in groups for arg in group)],
                              cwd=directory, capture_output=True, check=check)

    queue = run("--init-decisions", "sheet.tsv").stdout.decode().splitlines()
    queue_rows = [line.split("\t") for line in queue]
    actual_dispositions = {key: detail.rsplit("disposition=", 1)[1]
                           for kind, _, key, detail in queue_rows if kind == "source"}
    assert actual_dispositions == disposition
    candidate_groups = [keys.split(",") for kind, _, keys, _ in queue_rows if kind == "candidate"]
    mixed_groups = sum(len({copies[key].report for key in group}) > 1 for group in candidate_groups)
    sheet = directory / "sheet.tsv"
    template = sheet.read_bytes().decode().splitlines()
    assert [line.split("\t")[0] for line in template if line.endswith("\tpending")] == [
        key for key, decision in choice.items() if decision != "not-candidate"]

    def save(decisions):
        lines = []
        for line in template:
            key, _, value = line.partition("\t")
            lines.append(f"{key}\t{decisions[key]}" if value == "pending" else line)
        sheet.write_bytes(("\n".join(lines) + "\n").encode())

    save(choice)
    saved = sheet.read_bytes()
    run("--resume-decisions", "sheet.tsv")
    run("--export-reviewed", "sheet.tsv", "out")
    run("--export-reviewed", "sheet.tsv", "replay", reverse=True)
    ris = (directory / "out/reviewed.ris").read_bytes()
    mapping = (directory / "out/mapping.tsv").read_bytes()
    for name, data in (("reviewed.ris", ris), ("mapping.tsv", mapping)):
        assert (directory / "replay" / name).read_bytes() == data
    retained = [key for key, value in choice.items() if not value.startswith("duplicate-of=")]
    output_number = {key: n for n, key in enumerate(retained, 1)}
    assert ris == b"".join(canonical[key] for key in retained)
    rows = [row.split("\t") for row in mapping.decode().splitlines()]
    mapped = [row for row in rows if row[0] == "map"]
    assert [row[2] for row in mapped] == list(copies)
    for _, action, key, detail in mapped:
        span, number = detail.split(";output=")
        start, end = spans[key]
        assert action == choice[key] and span == f"bytes={start}..{end}"
        target = action.removeprefix("duplicate-of=") if action.startswith("duplicate-of=") else key
        assert number == str(output_number[target]) and copies[target].report == copies[key].report
    assert ["summary", "all", str(len(copies)),
            f"retained={len(retained)};suppressed={len(copies)-len(retained)}"] in rows
    assert ["output", "ris", "reviewed.ris",
            f"bytes={len(ris)};sha256={hashlib.sha256(ris).hexdigest()}"] in rows
    for label, data in original.items():
        assert (directory / f"{label}.ris").read_bytes() == data
        assert ["input", "ris", label,
                f"bytes={len(data)};occurrences={len(sources[label])};sha256={hashlib.sha256(data).hexdigest()}"] in rows
    assert sheet.read_bytes() == saved
    kept_truth = Counter(copies[key].report for key in retained)
    assert kept_truth.keys() == first.keys()
    actions = Counter(row[1].split("=")[0] for row in mapped)
    remaining = Counter()
    for key in retained:
        rec = copies[key]
        if key != first[rec.report]:
            remaining[choice[key] if rec.doi or rec.pmid else "no identifiers"] += 1
    print("Source occurrences:", {s: len(records) for s, records in sources.items()})
    print(f"Studies: 1000; distinct reports: {len(first)}; input occurrences: {len(copies)}; "
          f"input bytes: {sum(map(len, original.values()))}")
    print("Queue:", dict(Counter(actual_dispositions.values())))
    print(f"Candidate groups: {len(candidate_groups)}; groups spanning distinct reports: {mixed_groups}")
    print("Decisions:", dict(actions))
    print(f"Mapped: {len(mapped)}; retained: {len(retained)}; distinct reports lost: 0")
    print("True duplicate copies still retained:", dict(remaining))

    # One wrong edit targets the results article for a single-copy protocol with
    # a copied DOI and a contradictory PMID. Source identities are still valid.
    protocol = first[8, "protocol"]
    wrong = choice | {protocol: f"duplicate-of={first[8, 'results']}"}
    save(wrong)
    attempt = run("--export-reviewed", "sheet.tsv", "wrong", check=False)
    if attempt.returncode == 0:
        bad_ris = (directory / "wrong/reviewed.ris").read_bytes()
        assert bad_ris == b"".join(canonical[key] for key in retained if key != protocol)
        raise AssertionError(f"Contradictory decision accepted; report {copies[protocol].report} disappeared")
    assert b"PMID" in attempt.stderr and not (directory / "wrong").exists()
    undo = next(key for key in choice if choice[key].startswith("duplicate-of="))
    save(choice | {undo: "pending"})
    run("--export-reviewed", "sheet.tsv", "undo")
    expected_undo = b"".join(data for key, data in canonical.items()
                             if key == undo or not choice[key].startswith("duplicate-of="))
    assert (directory / "undo/reviewed.ris").read_bytes() == expected_undo
    save(choice)
    run("--export-reviewed", "sheet.tsv", "corrected")
    assert (directory / "corrected/reviewed.ris").read_bytes() == ris
    assert (directory / "corrected/mapping.tsv").read_bytes() == mapping
    assert sheet.read_bytes() == saved
    assert all((directory / f"{label}.ris").read_bytes() == data for label, data in original.items())
    print("Conflicting decision blocked; undo restored one occurrence; replay reproduced both files.")
    hashes = {"mapping.tsv": hashlib.sha256(mapping).hexdigest(), "reviewed.ris": hashlib.sha256(ris).hexdigest()}
    print(json.dumps({"audit": "synthetic_review", "mapped": len(mapped), "retained": len(retained),
                      "reports": len(kept_truth), "sha256": hashes}, sort_keys=True))
    assert hashes == {
        "mapping.tsv": "24d77de52827e6de72c33c960fc6f8f194bcfb3157c30b811d1f49fcb3c9d701",
        "reviewed.ris": "abcbffc592703278c06ca385ba4afd71a3e05ffe7e9f72b53e579d3d70746a0f",
    }, "Synthetic export hashes differ from the pinned fixture"


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path, help="Built dedup CLI executable")
    binary = parser.parse_args().binary.resolve()
    with tempfile.TemporaryDirectory(prefix="synthetic-review-") as tmp:
        exercise(Path(tmp), binary)
