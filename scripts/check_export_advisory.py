"""Exercise the guided confirmation with fictional report titles and saved decisions."""
import argparse
import subprocess
import tempfile
from pathlib import Path


def record(title, doi, fields=""):
    return f"TY  - JOUR\nTI  - {title}\nDO  - {doi}\n{fields}ER  -\n".encode()


def with_pmid(data, value):
    return data.replace(b"ER  -", f"UR  - https://pubmed.ncbi.nlm.nih.gov/{value}/\nER  -".encode())


def check(binary, reference, name, catalog, index, choices, expected_ris, wording=None, hard_error=None):
    with tempfile.TemporaryDirectory() as tmp:
        job = Path(tmp)
        sources = {"catalog.ris": catalog, "index.ris": index}
        for filename, data in sources.items():
            (job / filename).write_bytes(data)
        args = ["--ris-source", "catalog", "catalog.ris", "--ris-source", "index", "index.ris"]
        sheet = job / "choices.tsv"
        subprocess.run([binary, "review-ids", "--init-decisions", sheet, *args],
                       cwd=job, capture_output=True, check=True)
        saved = sheet.read_bytes()
        for key, choice in choices.items():
            before = f"{key}\tpending\n".encode()
            assert saved.count(before) == 1
            saved = saved.replace(before, f"{key}\t{choice}\n".encode())
        sheet.write_bytes(saved)

        def guide(destination, answer):
            answers = f"choices.tsv\ncatalog\ncatalog.ris\nindex\nindex.ris\n\nyes\n{destination}\n{answer}\n"
            result = subprocess.run([binary, "start"], input=answers.encode(), cwd=job, capture_output=True)
            assert sheet.read_bytes() == saved
            assert all((job / path).read_bytes() == data for path, data in sources.items())
            assert b"TI  -" not in result.stdout + result.stderr
            return result

        preview = guide("cancelled", "write" if hard_error else "")
        prompt = preview.stderr.decode()
        assert not (job / "cancelled").exists()
        if hard_error:
            assert preview.returncode == 2 and hard_error in prompt, prompt
            assert "Type write" not in prompt and "Title wording differs" not in prompt
            print(f"PASS {name} — exit 2 before confirmation, no export")
            return
        assert preview.returncode == 0 and "Export cancelled." in prompt, prompt
        assert prompt.count("Title wording differs") == int(wording is not None), prompt
        if wording:
            assert prompt.index(wording) < prompt.index("Type write"), prompt
            assert "literal labels in the same report" in prompt

        result = guide("reviewed", "write")
        assert result.returncode == 0, result.stderr
        if wording:
            confirmed = result.stderr.decode()
            assert confirmed.index(wording) < confirmed.index("Type write"), confirmed
        assert (job / "reviewed/reviewed.ris").read_bytes() == expected_ris
        subprocess.run([reference, "review-ids", "--export-reviewed", sheet, "reference", *args],
                       cwd=job, capture_output=True, check=True)
        for filename in ("reviewed.ris", "mapping.tsv", "reconciliation.tsv"):
            assert (job / "reviewed" / filename).read_bytes() == (job / "reference" / filename).read_bytes()
        count = expected_ris.count(b"TY  -")
        print(f"PASS {name} — {prompt.count('Title wording differs')} advisory, "
              f"{count} retained, cancellation wrote no export, three reference files matched")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--reference", type=Path, help="Use an earlier CLI for the reference export")
    parser.add_argument("--package-smoke", action="store_true",
                        help="Check the colon-caption pair and its conflicting-PMID control")
    args = parser.parse_args()
    cli = args.binary.resolve()
    reference = (args.reference or cli).resolve()
    copy = {"catalog:1": "keep", "index:1": "duplicate-of=catalog:1"}
    if not args.package_smoke:
        results = record("Lantern study results", "10.0000/lantern")
        protocol = record("Lantern study protocol", "10.0000/lantern")
        lantern = (results.replace(b"\n", b"\r\n"), results + protocol)
        check(cli, reference, "Lantern, protocol pending", *lantern, copy, results + protocol)
        check(cli, reference, "Lantern, protocol assigned", *lantern,
              {**copy, "index:2": "duplicate-of=catalog:1"}, results,
              "index:2 (remove) ends in 'protocol' and catalog:1 (keep) ends in 'results'")

    # Both catalogs describe one plate, using captions from different panels as titles.
    fields = ("AU  - Finch, O.\nPY  - 2024\nJO  - Fictional Type Specimens\nVL  - 3\n"
              "SP  - 18-19\nAN  - PLATE-42\nN1  - One specimen plate with two captioned panels "
              "showing the two menu labels.\n")
    separators = (": ",) if args.package_smoke else (" ", ": ")
    for separator in separators:
        heading = "Measuring the width of the menu label" + separator
        first = record(heading + "protocol", "10.0000/typeplate", fields)
        second = record(heading + "results", "10.0000/typeplate", fields)
        check(cli, reference, f"Specimen plate, separator {separator!r}", first, second, copy, first,
              "index:1 (remove) ends in 'results' and catalog:1 (keep) ends in 'protocol'")

        if separator == ": ":
            # The same pair with contradictory identifiers must fail even with a write answer.
            check(cli, reference, "Conflicting PMIDs", with_pmid(first, 111), with_pmid(second, 222),
                  copy, None, hard_error="Conflicting PMID")
