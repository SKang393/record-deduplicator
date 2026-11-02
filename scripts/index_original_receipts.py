"""Normalize dated collection receipt logs without changing originals or source logs.

This adapter targets the 2026-11-02 evidence schemas. The separate verifier
checks every actual file again. Browser receipt facts are not inferred from
filenames, a requested export, or a count-only observation.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

from build_collection_report import read


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("collection", type=Path)
    args = parser.parse_args()
    root = args.collection.resolve(strict=True)
    plan = read(root / "approved-export-plan.json")
    jobs = {j["export_id"]: j for j in plan.get("superseded_export_jobs", []) + plan["jobs"]}
    receipts, hashes = [], {}

    def load(name):
        path = root / "evidence" / name
        if not path.exists():
            return {}
        raw = path.read_bytes()
        hashes[name] = hashlib.sha256(raw).hexdigest()
        return json.loads(raw.decode("utf-8-sig"))

    name = "native-wos-exports.json"
    for source_job in load(name).get("jobs", []):
        job = jobs[source_job["export_id"]]
        checks = source_job.get("pre_export_rechecks", [])
        if not checks:
            continue
        recheck = checks[-1]
        for batch in source_job.get("batches", []):
            verified = batch.get("file_verification")
            delivered = batch.get("receipts", [])
            if not verified or not delivered:
                continue
            receipt = delivered[-1]
            receipts.append(dict(export_id=job["export_id"],
                relative_path=verified["preserved_relative_path"], format="RIS",
                downloaded_at_utc=receipt["received_at_utc"], sha256=verified["sha256"],
                byte_size=verified["bytes"], expected_records=verified["expected_records"],
                range_start=verified["range_start"], range_end=verified["range_end"],
                range_group="all", evidence_file=name,
                query_verified=recheck.get("exact_query") == job["query"],
                rechecked_source_count=recheck.get("result_count")))

    name = "native-biomedical-exports.json"
    bio = load(name)
    for verified in bio.get("file_verification", []):
        export_id = verified["export_id"]
        job = jobs[export_id]
        rechecks = [r for r in bio.get("events", [])
                    if r.get("kind") == "native_export_recheck" and r.get("export_id") == export_id]
        delivered = [r for r in bio.get("receipts", []) if r.get("export_id") == export_id]
        if not rechecks or not delivered:
            continue
        # Component/batched exports need explicit file-to-receipt identity.
        matches = [r for r in delivered if r.get("original_filename") == verified.get("original_filename")]
        if len(matches) != 1:
            raise ValueError(f"Ambiguous native receipt mapping: {export_id}")
        receipt, recheck = matches[0], rechecks[-1]
        receipts.append(dict(export_id=export_id,
            relative_path=verified["preserved_relative_path"], format=job["format"],
            downloaded_at_utc=receipt["download_completed_at_utc"],
            sha256=verified["sha256"], byte_size=verified["bytes"],
            expected_records=receipt["selected_count"], evidence_file=name,
            range_start=receipt.get("range_start", receipt.get("component_range_start")),
            range_end=receipt.get("range_end", receipt.get("component_range_end")),
            range_group=receipt.get("record_type_component", "all"),
            query_verified=recheck.get("exact_query") == job["query"],
            rechecked_source_count=recheck.get("count")))

    name = "expanded-download-receipts.json"
    downloads = load(name)
    requests = {j["export_id"]: j for j in load("ebsco-expanded-exports.json").get("jobs", [])}
    for receipt in downloads.get("receipts", []):
        export_id = receipt["export_id"]
        job, request = jobs[export_id], requests[export_id]
        mail = read(root / "evidence" / f"email-{receipt['email_id']}.json")
        path = Path(receipt["project_copy_path"]).resolve(strict=True)
        query = receipt.get("query", downloads.get("source_query"))
        required_settings = job.get("required_settings_evidence")
        settings_proof = request.get("recheck", {}).get("settings", "")
        settings_verified = (not required_settings or
                             ("explicit" in settings_proof.lower() and "apply" in settings_proof.lower()))
        receipts.append(dict(export_id=export_id, relative_path=path.relative_to(root).as_posix(),
            format="RIS", downloaded_at_utc=receipt["browser_download_completed_utc"],
            sha256=receipt["sha256"], byte_size=receipt["bytes"],
            expected_records=job["expected_raw_count"], evidence_file=name,
            query_verified=(query == job["query"] and job["query"] in mail["body_text"] and
                            request["query"] == job["query"] and
                            request.get("status") == "acknowledged_export_requested" and settings_verified),
            rechecked_source_count=request.get("recheck", {}).get("count"),
            database_tag_counts=receipt.get("ris_db_tag_counts"),
            source_email_id=receipt["email_id"]))
    result = dict(indexed_at_utc=datetime.now(timezone.utc).isoformat(),
                  scope="Normalized receipt facts; actual file verification is a separate command",
                  input_sha256=hashes, receipts=receipts)
    (root / "evidence" / "expanded-receipts.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(dict(receipts=len(receipts), input_files=list(hashes)), indent=2))


if __name__ == "__main__":
    main()
