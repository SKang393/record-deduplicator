"""Edit a fictional review at the guide prompts and check which preview can be written."""
import argparse
import subprocess
import tempfile
import time
from pathlib import Path


def read_prompt(log, process, marker):
    # Read only new prompts. Wait for the actual question before editing the sheet.
    seen = b""
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        seen += log.read()
        if marker in seen:
            return seen
        if process.poll() is not None:
            break
        time.sleep(0.01)
    raise AssertionError(f"Missing {marker!r} in {seen!r}")


def check(binary, job):
    def record(accession):
        return f"TY  - JOUR\nTI  - Fictional river survey\nDO  - 10.0000/river\nAN  - {accession}\nER  -\n".encode()

    cat, idx = record("C1"), record("I1") + record("I2")
    (job / "cat.ris").write_bytes(cat)
    (job / "idx.ris").write_bytes(idx)
    sources = ["--ris-source", "cat", "cat.ris", "--ris-source", "idx", "idx.ris"]

    def review(*args):
        subprocess.run([binary, "review-ids", *args, *sources], cwd=job, check=True, capture_output=True)

    sheet = job / "choices.tsv"
    review("--init-decisions", sheet)
    pending = sheet.read_bytes().replace(b"cat:1\tpending", b"cat:1\tkeep")
    one_removed = pending.replace(b"idx:1\tpending", b"idx:1\tduplicate-of=cat:1")
    two_removed = one_removed.replace(b"idx:2\tpending", b"idx:2\tduplicate-of=cat:1")
    assert two_removed != one_removed != pending
    question = b"Type write to create this export"
    for outcome in ("cancel", "write", "invalid", "inspect"):
        sheet.write_bytes(one_removed if outcome == "inspect" else two_removed)
        log_path = job / "prompts.txt"
        with log_path.open("wb") as stderr, log_path.open("rb") as log, subprocess.Popen(
            [binary, "start"], cwd=job, stdin=subprocess.PIPE, bufsize=0,
            stdout=subprocess.PIPE, stderr=stderr,
        ) as guide:
            try:
                guide.stdin.write(b"choices.tsv\ncat\ncat.ris\nidx\nidx.ris\n\nyes\n")
                read_prompt(log, guide, b"Press Enter to close this session:")
                if outcome == "inspect":
                    # An equivalent spreadsheet save can still be inspected. Moving the
                    # removal to another row keeps the byte count but changes the decisions.
                    bom = b"\xef\xbb\xbf"
                    sheet.write_bytes(bom + one_removed)
                    guide.stdin.write(b":leads\n")
                    listed = read_prompt(log, guide, b"Press Enter to close this session:")
                    assert b"Inspection-only pairs: 0." in listed, listed
                    sheet.write_bytes(one_removed)
                    guide.stdin.write(b":inspect cat:1,idx:1\n")
                    read_prompt(log, guide, b"Press Enter to close this session:")
                    edited = pending.replace(b"idx:2\tpending", b"idx:2\tduplicate-of=cat:1")
                    assert len(edited) == sheet.stat().st_size
                    # Change and restore the exact sheet while the guide is waiting.
                    sheet.write_bytes(edited)
                    sheet.write_bytes(one_removed)
                    guide.stdin.write(b":inspect cat:1,idx:2\n")
                    read_prompt(log, guide, b"Press Enter to close this session:")
                    sheet.write_bytes(edited)
                    output, _ = guide.communicate(b":inspect cat:1,idx:2\n:pending\n\n", timeout=10)
                    assert guide.returncode == 0 and b"Reloaded the saved decision sheet." in log.read()
                    assert output.count(b"Citation fields (normalized,") == 3
                    assert b"Citation fields (normalized, cat:1 | idx:1)" in output
                    assert b"[idx:2] pending (bytes " in output
                    assert b"[idx:2] duplicate-of=cat:1 (bytes " in output
                    current_list = subprocess.check_output(
                        [binary, "review-ids", "--pending-decisions", sheet, *sources], cwd=job
                    )
                    assert output.endswith(current_list)
                    assert sheet.read_bytes() == edited
                    print("PASS: inspection and :pending reflected the same-size saved edit in the open session.")
                    continue
                sheet.write_bytes(one_removed)
                guide.stdin.write(outcome.encode() + b"\n")
                preview = read_prompt(log, guide, question)
                assert b"Explicitly marked duplicates to remove: 1" in preview, preview
                assert b"Records to export for screening: 2 (including 1 pending duplicate review)." in preview, preview
                edited = pending if outcome != "invalid" else pending.replace(b"idx:1\tpending", b"idx:1\toops")
                sheet.write_bytes(edited)
                guide.stdin.write(b"write\n")
                if outcome == "invalid":
                    guide.communicate(timeout=10)
                    error = log.read()
                    assert guide.returncode == 2 and b"Unknown decision for idx:1" in error, error
                    assert question not in error and b"Records to export for screening:" not in error, error
                else:
                    refreshed = read_prompt(log, guide, question)
                    assert b"Saved decisions changed after the preview" in refreshed, refreshed
                    assert b"Explicitly marked duplicates to remove: 0" in refreshed, refreshed
                    assert b"Records to export for screening: 3 (including 2 pending duplicate review)." in refreshed, refreshed
                    assert not (job / outcome).exists()
                    if outcome == "cancel":
                        # Restoring the first preview cannot resurrect its earlier approval.
                        edited = one_removed
                        sheet.write_bytes(edited)
                        guide.stdin.write(b"write\n")
                        restored = read_prompt(log, guide, question)
                        assert b"Saved decisions changed after the preview" in restored, restored
                        assert b"Explicitly marked duplicates to remove: 1" in restored, restored
                        assert not (job / outcome).exists()
                        print("PASS: restoring the first preview required fresh confirmation and wrote no export.")
                    guide.communicate(b"write\n" if outcome == "write" else b"\n", timeout=10)
                    assert guide.returncode == 0, log.read()
            finally:
                if guide.poll() is None:
                    guide.kill()
        assert sheet.read_bytes() == edited
        assert (job / "cat.ris").read_bytes() == cat and (job / "idx.ris").read_bytes() == idx
        if outcome == "write":
            review("--export-reviewed", sheet, "reference")
            assert (job / "write/reviewed.ris").read_bytes() == cat + idx
            mapping = (job / "write/mapping.tsv").read_bytes()
            assert b"map\tpending\tidx:1\t" in mapping and b"map\tpending\tidx:2\t" in mapping
            for name in ("reviewed.ris", "mapping.tsv", "reconciliation.tsv"):
                assert (job / "write" / name).read_bytes() == (job / "reference" / name).read_bytes()
        else:
            assert not (job / outcome).exists()

    print("PASS: preview used the first edit (1 removal, 1 pending); a later edit required new approval "
          "(0 removals, 2 pending). Enter cancelled; a fresh write matched all three explicit export files. "
          "Invalid edits refused. Sources and edited sheet preserved.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as tmp:
        check(args.binary.resolve(strict=True), Path(tmp))
