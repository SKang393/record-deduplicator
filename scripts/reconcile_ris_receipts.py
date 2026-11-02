"""Reconcile original RIS receipts with observed export requests and count evidence.

Run verify_ris_exports.py first and save its JSON under evidence/ris-integrity.json.
No source file is rewritten. This does not test duplicate detection.
"""

import argparse
from collections import Counter
import csv
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

from build_collection_report import eastern, read


def canonical(name):
    return name.casefold().replace("\u00ae", "").strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("collection", type=Path)
    parser.add_argument("public_output", type=Path)
    args = parser.parse_args()
    evidence = args.collection / "evidence"
    integrity = read(evidence / "ris-integrity.json")
    receipts = read(evidence / "download-receipts.json")["downloads"]
    requests = []
    for p in evidence.glob("*-search-trials.json"):
        log = read(p)
        requests.extend(dict(export, field=log["field"]) for export in log.get("exports", []))
    reconciled, public, profiles = [], [], []
    for entry in integrity["files"]:
        rel = Path(entry["path"])
        file = args.collection / "ris" / rel
        downloaded = [r for r in receipts if r["original_filename"] == file.name]
        assert len(downloaded) == 1, (file.name, "Ambiguous receipt")
        receipt = downloaded[0]
        assert Path(receipt["final_path"]).resolve() == file.resolve(), (file.name, "Receipt path mismatch")
        assert receipt["size_bytes"] == entry["bytes"], (file.name, "Receipt size mismatch")
        raw = file.read_bytes()
        assert len(raw) == entry["bytes"] and hashlib.sha256(raw).hexdigest() == entry["sha256"], (file.name, "Stale integrity report")
        content = raw.decode(entry["encoding_for_counting"])
        lines = content.splitlines()
        database = Counter(line[6:].strip() for line in lines if line.startswith("DB  - "))
        tags = Counter(line[:2] for line in lines if re.match(r"^[A-Z0-9]{2}  -", line))
        field = rel.parts[0]
        candidates = [r for r in requests if r["field"] == field and len(database) == 1
                      and canonical(r["database"]) == canonical(next(iter(database)))]
        export = candidates[0] if len(candidates) == 1 else None
        mail = read(evidence / f"email-{receipt['source_email_id']}.json")
        query_match = bool(export and export.get("query") and export["query"] in mail["body_text"])
        single_db = len(database) == 1 and sum(database.values()) == entry.get("ty_count")
        status = "needs_mapping_review"
        if export and query_match and single_db and entry["status"] == "boundaries_ok":
            status = ("original_received_reconciled" if export["requested_count"] == entry["ty_count"]
                      else "original_received_count_difference_requires_review")
        record = dict(field=field, corpus_role="preserved_ebsco_only_pilot", export_id=export["id"] if export else None,
                      database=next(iter(database)) if len(database) == 1 else None,
                      status=status, requested_count=export.get("requested_count") if export else None,
                      delivered_record_count=entry.get("ty_count"),
                      requested_minus_delivered=(export["requested_count"] - entry["ty_count"]) if export else None,
                      byte_size=entry["bytes"], sha256=entry["sha256"], ty_count=entry.get("ty_count"),
                      er_count=entry.get("er_count"), encoding=entry.get("encoding_for_counting"),
                      verified_at_utc=integrity["checked_at_utc"],
                      verified_at_eastern=eastern(integrity["checked_at_utc"]),
                      downloaded_at_utc=receipt["download_completed_utc"],
                      email_received_at_utc=datetime.fromtimestamp(int(mail["gmail_received_internal_date"]) / 1000, timezone.utc).isoformat(),
                      email_query_matches_request=query_match, database_tag_counts=dict(database))
        public.append(record)
        reconciled.append(dict(record, relative_path=entry["path"], source_email_id=receipt["source_email_id"],
                               original_filename=receipt["original_filename"], observed_link=receipt["observed_link"]))
        profiles.extend(dict(field=field, export_id=record["export_id"], tag=tag, occurrences=n)
                        for tag, n in sorted(tags.items()))
    private_result = dict(checked_at_utc=integrity["checked_at_utc"],
                          scope="Receipt, query, DB tags, immutable-byte inventory and record boundaries only",
                          files=reconciled)
    (evidence / "export-reconciliation.json").write_text(json.dumps(private_result, indent=2) + "\n", encoding="utf-8")
    args.public_output.mkdir(parents=True, exist_ok=True)
    if public:
        for name, rows in (("received-exports.csv", public), ("ris-tag-profile.csv", profiles)):
            with (args.public_output / name).open("w", encoding="utf-8", newline="") as handle:
                writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
                writer.writeheader()
                writer.writerows({k: json.dumps(v) if isinstance(v, dict) else v for k, v in row.items()} for row in rows)
    manifest_path = args.collection / "collection-manifest.json"
    manifest = read(manifest_path)
    manifest["exports"] = reconciled
    manifest["exports_role"] = "preserved_ebsco_only_pilots; expanded_exports is separate"
    manifest["pilot_exports"] = reconciled
    manifest["last_reconciled_at_utc"] = integrity["checked_at_utc"]
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    events_path = args.collection / "events.jsonl"
    prior = [json.loads(line) for line in events_path.read_text(encoding="utf-8-sig").splitlines() if line.strip()]
    prior_ids = {x.get("event_id", x.get("id")) for x in prior}
    with events_path.open("a", encoding="utf-8") as handle:
        for row in reconciled:
            ident = "EV-RIS-" + row["sha256"][:20]
            if ident in prior_ids:
                continue
            event = dict(event_id=ident, observed_at_utc=integrity["checked_at_utc"],
                         observed_at_eastern=eastern(integrity["checked_at_utc"]),
                         actor_tool="reconcile_ris_receipts.py", action="original_ris_receipt_reconciled",
                         source="evidence/ris-integrity.json and download-receipts.json",
                         field=row["field"], export_id=row["export_id"], result=row["status"],
                         sha256=row["sha256"], evidence_path="evidence/export-reconciliation.json")
            handle.write(json.dumps(event) + "\n")
    print(json.dumps(dict(files=len(public), records=sum(r["delivered_record_count"] for r in public),
                          statuses=dict(Counter(r["status"] for r in public))), indent=2))
    if not public:
        return 2
    return 0 if all(r["status"] == "original_received_reconciled" for r in public) else 1


if __name__ == "__main__":
    raise SystemExit(main())
