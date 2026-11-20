"""Follow six generated RIS occurrences through queue, sheet and reviewed export."""
import argparse
import subprocess
import tempfile
from pathlib import Path


def exercise(directory, binary, baseline):
    # Report A has two exports. B is on different pages, C has a different
    # year, D carries contradictory DOIs and E shares A's DOI but has other pages.
    # Those four are distinct reports by construction.
    def record(accession, pages="21-26", year="2021", extra=""):
        return (f"TY  - JOUR\nAN  - {accession}\nTI  - Marsh transects\nAU  - Reed, E.\n"
                f"AU  - Moss, T.\nPY  - {year}\nJO  - Fable Ecology\nVL  - 8\n"
                f"SP  - {pages}\n{extra}ER  -\n").encode()

    a = record("cat-a", extra="DO  - 10.0000/marsh-a\n")
    b = record("cat-b", pages="27-31")
    e = record("cat-e", pages="40-42", extra="DO  - 10.0000/marsh-a\n")
    copy = (record("idx-a").replace(b"JO  -", b"JF  -").replace(b"AU  -", b"A1  -")
            .replace(b"SP  - 21-26", b"SP  - 21\nEP  - 26"))
    c = record("idx-c", year="2022")
    d = record("idx-d", extra="DO  - 10.0000/east\nDO  - 10.0000/west\n")
    catalog = directory / "cat.ris"
    index = directory / "idx.ris"
    catalog.write_bytes(a + b + e)
    index.write_bytes((copy + c + d).replace(b"\n", b"\r\n"))
    original = (catalog.read_bytes(), index.read_bytes())
    sources = ["--ris-source", "cat", catalog, "--ris-source", "idx", index]

    def run(*args, ok=True):
        result = subprocess.run([binary, "review-ids", *args, *sources], cwd=directory, capture_output=True)
        if ok:
            assert result.returncode == 0, result.stderr
        else:
            assert result.returncode != 0 and not result.stdout
        return result

    def output(name, file):
        return (directory / name / file).read_bytes()

    sheet = directory / "choices.tsv"
    queue = run("--init-decisions", sheet).stdout.decode().splitlines()
    assert "triage\ttitle_author_year_type\tcat:1,cat:2,cat:3,idx:1,idx:3\tinspect_metadata_only" in queue
    legacy = sheet.read_bytes()
    assert [line for line in legacy.splitlines() if line.endswith(b"\tpending")] == [
        b"cat:1\tpending", b"cat:3\tpending"]
    run("--export-reviewed", sheet, "untouched")
    assert output("untouched", "reviewed.ris") == a + b + e + copy + c + d
    sheet.write_bytes(legacy.replace(b"end\n", b"idx:1\tduplicate-of=cat:1\nend\n"))
    run("--export-reviewed", sheet, "invalid", ok=False)
    assert not (directory / "invalid").exists()
    print("Baseline: 5 records in one metadata hint, 2 editable ID rows, 0 editable ID-less copies; 6 retained.")
    if baseline:
        return

    sheet = directory / "metadata.tsv"
    assert run("--init-metadata-decisions", sheet).stdout.decode().splitlines() == queue
    template = sheet.read_bytes()
    eligible = [line.split(b"\t")[0] for line in template.splitlines() if line.endswith(b"\tpending")]
    assert eligible == [b"cat:1", b"cat:2", b"cat:3", b"idx:1", b"idx:3"]
    run("--export-reviewed", sheet, "pending")
    assert output("pending", "reviewed.ris") == a + b + e + copy + c + d

    reason = "Both publications list Reed and Moss, Fable Ecology 8, pages 21–26.".encode()
    verified = b"verified-duplicate-of=cat:1\t" + reason
    kept = template.replace(b"cat:1\tpending", b"cat:1\tkeep")
    approved = kept.replace(b"idx:1\tpending", b"idx:1\t" + verified)
    assert approved != kept != template

    def rejected(edits, diagnostic):
        sheet.write_bytes(edits)
        for args in [("--resume-decisions", sheet), ("--export-reviewed", sheet, "invalid")]:
            assert diagnostic in run(*args, ok=False).stderr, diagnostic
        assert not (directory / "invalid").exists()

    rejected(approved.replace(reason, b"  "), b"rationale")
    rejected(approved.replace(reason, b"extra\tcolumn"), b"rationale")
    rejected(approved.replace(b"\tkeep\n", b"\tpending\n"), b"keep")
    rejected(approved.replace(verified, b"duplicate-of=cat:1"), b"direct candidate")
    rejected(kept.replace(b"cat:2\tpending", b"cat:2\t" + verified), b"final pages")
    rejected(kept.replace(b"idx:3\tpending", b"idx:3\t" + verified), b"Conflicting DOI")
    rejected(approved.replace(b"cat:3\tpending", b"cat:3\tduplicate-of=cat:1"), b"final pages")
    rejected(approved.replace(b"end\n", b"idx:2\t" + verified + b"\nend\n"), b"unexpected rows")
    index.write_bytes(original[1].replace(b"idx-a", b"idx-z"))
    rejected(approved, b"inputs differ")
    index.write_bytes(original[1])

    sheet.write_bytes(approved)
    preview = run("--resume-decisions", sheet).stdout.splitlines()
    rationale = b"rationale\tmetadata\tidx:1\t" + reason
    assert b"decision\thuman\tidx:1\tverified-duplicate-of=cat:1" in preview and rationale in preview
    run("--export-reviewed", sheet, "reviewed")
    assert output("reviewed", "reviewed.ris") == a + b + e + c + d
    mapping = output("reviewed", "mapping.tsv").splitlines()
    assert b"schema\texport\t2\treviewed-ris" in mapping and mapping.count(rationale) == 1
    assert b"summary\tall\t6\tretained=5;suppressed=1" in mapping
    expected = [(b"cat:1", b"keep", 1), (b"cat:2", b"pending", 2), (b"cat:3", b"pending", 3),
                (b"idx:1", b"verified-duplicate-of=cat:1", 1), (b"idx:2", b"not-candidate", 4),
                (b"idx:3", b"pending", 5)]
    rows = [line.split(b"\t") for line in mapping if line.startswith(b"map\t")]
    assert [(key, action, int(detail.split(b";output=")[1])) for _, action, key, detail in rows] == expected
    copy_bytes = len(copy.replace(b"\n", b"\r\n"))
    assert rows[3][3] == f"bytes=0..{copy_bytes};output=1".encode()
    assert sheet.read_bytes() == approved
    sheet.write_bytes(kept)
    run("--export-reviewed", sheet, "undo")
    assert output("undo", "reviewed.ris") == output("pending", "reviewed.ris")
    sheet.write_bytes(approved)
    run("--export-reviewed", sheet, "replay")
    for name in ["reviewed.ris", "mapping.tsv"]:
        assert output("replay", name) == output("reviewed", name)
    assert (catalog.read_bytes(), index.read_bytes()) == original
    print("Verified sheet: 5 editable, 6 mapped, 1 verified copy, all 5 distinct reports retained.")
    print("Nine invalid edits/input changes rejected before output. Undo restored all 6; replay matched both files.")

    # Change one corroborating field at a time. These variants still have the
    # original title/first author/year/type hint and fresh source fingerprints.
    variants = [
        copy.replace(b"Moss, T.", b"Elm, C."),
        copy.replace(b"Fable Ecology", b"Fable Forestry"),
        copy.replace(b"VL  - 8", b"VL  - 9"),
        copy.replace(b"SP  - 21\nEP  - 26", b"SP  - Online first"),
        copy.replace(b"ER  -", b"IS  - 2\nER  -"),
        copy.replace(b"ER  -", b"M3  - Study protocol\nER  -"),
    ]
    for num, altered in enumerate(variants):
        catalog.write_bytes(a)
        index.write_bytes(altered)
        sheet = directory / f"variant-{num}.tsv"
        run("--init-metadata-decisions", sheet)
        edits = sheet.read_bytes().replace(b"cat:1\tpending", b"cat:1\tkeep")
        assert b"idx:1\tpending" in edits
        rejected(edits.replace(b"idx:1\tpending", b"idx:1\t" + verified), b"publication details")
    print(f"All {len(variants)} conflicting or incomplete publication-detail variants stayed blocked.")

    # Two copies point forward to an ID-less keeper. Each has the same
    # article locator as the keeper, but their supported DOIs contradict.
    later = record("other", pages="e31415", extra="DO  - 10.0000/other\n")
    article = a.replace(b"21-26", b"e31415")
    catalog.write_bytes(article + later)
    keeper = record("keeper", pages="e31415")
    index.write_bytes(keeper)
    sheet = directory / "article.tsv"
    run("--init-metadata-decisions", sheet)
    saved = sheet.read_bytes().replace(b"idx:1\tpending", b"idx:1\tkeep")
    link = b"verified-duplicate-of=idx:1\tCompared the full publications and article number e31415."
    one = saved.replace(b"cat:1\tpending", b"cat:1\t" + link)
    rejected(one.replace(b"cat:2\tpending", b"cat:2\t" + link), b"Conflicting DOI")
    sheet.write_bytes(one)
    run("--export-reviewed", sheet, "article")
    assert output("article", "reviewed.ris") == later + keeper
    assert b";output=2" in next(row for row in output("article", "mapping.tsv").splitlines()
                                if row.startswith(b"map\tverified-duplicate-of=idx:1\tcat:1\t"))
    print("Shared article number allowed the verified pair; discordant sibling DOIs blocked the three-record set.")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--baseline", action="store_true")
    options = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="metadata-decisions-") as tmp:
        exercise(Path(tmp), options.binary.resolve(), options.baseline)
