"""Reconcile approved source jobs with original RIS/NBIB download receipts.

Collection integrity only, not a production parser or deduplication evaluation.
The private expanded-receipts.json is a normalized index of browser receipts.
Each receipt declares export_id, relative_path, format, downloaded_at_utc,
sha256, byte_size, expected_records, evidence_file, query_verified,
rechecked_source_count, and optional range_start/range_end/range_group.
Original files are never rewritten. Unreceived jobs remain incomplete.
"""

import argparse
from collections import Counter, defaultdict
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

from build_collection_report import read, write_csv, eastern
from verify_ris_exports import inspect as inspect_ris


def inspect_nbib(path, root):
    raw = path.read_bytes()
    result = dict(path=path.relative_to(root).as_posix(), bytes=len(raw),
                  sha256=hashlib.sha256(raw).hexdigest(), errors=[], warnings=[])
    try:
        content = raw.decode("utf-8-sig")
    except UnicodeDecodeError as error:
        return dict(result, status="decode_error", errors=[str(error)], record_count=0)
    blocks = [s for s in re.split(r"\r?\n\s*\r?\n", content.strip()) if s.strip()]
    pmids = []
    for number, block in enumerate(blocks, 1):
        first = block.splitlines()[0]
        match = re.fullmatch(r"PMID-\s+(\d+)\s*", first)
        if not match or len(re.findall(r"^PMID-", block, re.MULTILINE)) != 1:
            result["errors"].append(f"NBIB block {number} lacks exactly one leading numeric PMID")
        else:
            pmids.append(match.group(1))
    if not blocks:
        result["errors"].append("No NBIB records found")
    result.update(record_count=len(blocks), encoding_for_counting="utf-8-sig",
                  repeated_pmid_count=len(pmids) - len(set(pmids)),
                  status="boundaries_ok" if not result["errors"] else "boundary_error")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("collection", type=Path)
    parser.add_argument("public_output", type=Path)
    args = parser.parse_args()
    root = args.collection.resolve(strict=True)
    plan = read(root / "approved-export-plan.json")
    search_plan = read(root / "expanded-search-plan.json")
    receipts_path = root / "evidence" / "expanded-receipts.json"
    receipts = read(receipts_path).get("receipts", []) if receipts_path.exists() else []
    active_jobs = {j["export_id"]: j for j in plan["jobs"]}
    jobs = {j["export_id"]: j for j in plan.get("superseded_export_jobs", []) + plan["jobs"]}
    if len(active_jobs) != len(plan["jobs"]):
        raise ValueError("Duplicate export IDs in approved plan")
    now = datetime.now(timezone.utc).isoformat()
    files, by_job, seen_paths = [], defaultdict(list), set()
    for receipt in receipts:
        job = jobs.get(receipt["export_id"])
        if job is None:
            raise ValueError(f"Unapproved export ID: {receipt['export_id']}")
        path = (root / receipt["relative_path"]).resolve(strict=True)
        path.relative_to(root / "originals")
        path.relative_to((root / job["output_directory"]).resolve())
        if path in seen_paths:
            raise ValueError("The same local file is registered more than once")
        seen_paths.add(path)
        kind = receipt["format"].upper()
        if kind == "RIS":
            inspection = inspect_ris(path, root)
            inspection["record_count"] = inspection.get("ty_count", 0)
        elif kind == "NBIB":
            inspection = inspect_nbib(path, root)
        else:
            raise ValueError(f"Unsupported declared original format: {kind}")
        problems = list(inspection.get("errors", []))
        checks = {
            "boundary structure": inspection["status"] == "boundaries_ok",
            "download hash": receipt.get("sha256") == inspection["sha256"],
            "download byte size": receipt.get("byte_size") == inspection["bytes"],
            "expected batch count": receipt.get("expected_records") == inspection["record_count"],
            "source query verification": receipt.get("query_verified") is True,
            "rechecked source count": receipt.get("rechecked_source_count") == job["expected_raw_count"],
            "download timestamp": bool(receipt.get("downloaded_at_utc")),
            "receipt evidence": bool(receipt.get("evidence_file")) and
                (root / "evidence" / receipt["evidence_file"]).is_file(),
            "approved format": kind == job["format"].upper(),
        }
        problems.extend(key for key, okay in checks.items() if not okay)
        if job["source_id"].startswith("EBSCO:") and inspection["status"] == "boundaries_ok":
            content = path.read_bytes().decode(inspection["encoding_for_counting"])
            databases = Counter(line[6:].strip() for line in content.splitlines()
                                if line.startswith("DB  - "))
            if dict(databases) != receipt.get("database_tag_counts"):
                problems.append("Actual database tags differ from the browser receipt")
            if len(databases) != 1 or sum(databases.values()) != inspection["record_count"]:
                problems.append("EBSCO file lacks one source database tag per record")
        start, end = receipt.get("range_start"), receipt.get("range_end")
        if start is not None or end is not None:
            if not (isinstance(start, int) and isinstance(end, int) and
                    1 <= start <= end and end - start + 1 == inspection["record_count"]):
                problems.append("Invalid or count-inconsistent export range")
        row = dict(export_id=receipt["export_id"], field=job["field"],
                   trial_id=job["trial_id"], source_id=job["source_id"],
                   corpus_role=("selected_collection_original" if receipt["export_id"] in active_jobs
                                else job.get("corpus_role", "superseded_not_final")),
                   format=kind, record_count=inspection["record_count"],
                   byte_size=inspection["bytes"], sha256=inspection["sha256"],
                   downloaded_at_utc=receipt.get("downloaded_at_utc"),
                   downloaded_at_eastern=eastern(receipt.get("downloaded_at_utc")),
                   range_start=start, range_end=end,
                   range_group=receipt.get("range_group", "all"),
                   status="verified_original_receipt" if not problems else "requires_review",
                   problems=problems, warnings=inspection.get("warnings", []))
        files.append(dict(row, relative_path=receipt["relative_path"],
                          evidence_file=receipt["evidence_file"], inspection=inspection))
        if row["export_id"] in active_jobs:
            by_job[row["export_id"]].append(row)
    summaries = []
    for export_id, job in active_jobs.items():
        batch_rows = by_job[export_id]
        errors = []
        counts = sum(r["record_count"] for r in batch_rows)
        if any(r["status"] != "verified_original_receipt" for r in batch_rows):
            errors.append("A receipt failed file/provenance checks")
        if len({r["sha256"] for r in batch_rows}) != len(batch_rows):
            errors.append("Repeated identical file bytes within one source job")
        if counts > job["expected_raw_count"]:
            errors.append("Delivered records exceed approved source count")
        groups = defaultdict(list)
        for row in batch_rows:
            groups[row["range_group"]].append(row)
        for group, rows in groups.items():
            ranged = [r for r in rows if isinstance(r["range_start"], int)]
            if len(rows) > 1 and len(ranged) != len(rows):
                errors.append(f"Multiple batches lack complete range evidence in {group}")
            if ranged:
                next_start = 1
                for row in sorted(ranged, key=lambda r: r["range_start"]):
                    if row["range_start"] != next_start:
                        errors.append(f"Gap or overlap in {group} range coverage")
                    next_start = row["range_end"] + 1
        complete = counts == job["expected_raw_count"] and not errors and bool(batch_rows)
        summaries.append(dict(export_id=export_id, field=job["field"], trial_id=job["trial_id"],
                              source_id=job["source_id"], expected_raw_count=job["expected_raw_count"],
                              received_records=counts, received_files=len(batch_rows),
                              status="source_complete" if complete else "incomplete_or_requires_review",
                              issues=errors))
    fields = []
    for field in plan["fields"]:
        selected = [r for r in summaries if r["field"] == field["field"] and r["trial_id"] == field["trial_id"]]
        total = sum(r["received_records"] for r in selected)
        admitted = (len(selected) == field["source_count"] and
                    all(r["status"] == "source_complete" for r in selected) and
                    total == field["raw_total_at_approval"] and 10000 <= total <= 20000)
        fields.append(dict(field=field["field"], role=field["role"], trial_id=field["trial_id"],
                           approved_raw_total=field["raw_total_at_approval"], received_records=total,
                           complete_sources=sum(r["status"] == "source_complete" for r in selected),
                           expected_sources=field["source_count"], corpus_admitted=admitted))
    approved_fields = {f["field"] for f in fields}
    for field in search_plan["fields"]:
        if field["field"] not in approved_fields:
            fields.append(dict(field=field["field"], role=field["role"], trial_id=None,
                               approved_raw_total=None, received_records=0, complete_sources=0,
                               expected_sources=len(field["source_ids"]), corpus_admitted=False,
                               status="query_not_yet_approved_for_export"))
    result = dict(checked_at_utc=now, execution_status=plan.get("status"),
                  scope="Existing local provenance, bytes and record counts only; no acquisition or matching/model validation",
                  files=files, sources=summaries, fields=fields)
    (root / "evidence" / "original-export-integrity.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    args.public_output.mkdir(parents=True, exist_ok=True)
    if files:
        public = [{k: v for k, v in r.items() if k not in {"relative_path", "evidence_file", "inspection"}} for r in files]
        write_csv(args.public_output / "expanded-received-exports.csv", list(public[0]), public)
    write_csv(args.public_output / "expanded-export-status.csv", list(summaries[0]), summaries)
    (args.public_output / "expanded-corpus-status.json").write_text(json.dumps(
        dict(checked_at_utc=now, execution_status=plan.get("status"), fields=fields),
        indent=2) + "\n", encoding="utf-8")
    manifest_path = root / "collection-manifest.json"
    manifest = read(manifest_path)
    manifest.update(expanded_exports=files, expanded_fields=fields,
                    final_admitted_fields=[f["field"] for f in fields if f["corpus_admitted"]],
                    last_expanded_integrity_check_utc=now)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(dict(files=len(files), records=sum(r["record_count"] for r in files),
                          receipt_statuses=dict(Counter(r["status"] for r in files)), fields=fields), indent=2))
    if any(r["problems"] for r in files) or any(r["issues"] for r in summaries):
        return 1
    return 0 if fields and all(r["corpus_admitted"] for r in fields) else 2


if __name__ == "__main__":
    raise SystemExit(main())
