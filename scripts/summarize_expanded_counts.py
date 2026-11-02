"""Index observed source-native searches without accepting a partial field total."""

import argparse
from collections import defaultdict
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

from build_collection_report import eastern, read, write_csv


ACCEPTED = {"observed", "observed_explicitly_applied", "measured", "measured_with_warning", "count_verified", "verified", "completed"}
LOGS = ["ebsco-expanded-counts.json", "native-biomedical-counts.json",
        "native-crossdisciplinary-counts.json", "native-wos-counts.json"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("collection", type=Path)
    parser.add_argument("public_output", type=Path)
    args = parser.parse_args()
    plan = read(args.collection / "expanded-search-plan.json")
    observations, hashes = [], {}
    grouped = defaultdict(list)
    for name in LOGS:
        path = args.collection / "evidence" / name
        if not path.exists():
            continue
        raw = path.read_bytes()
        hashes[name] = hashlib.sha256(raw).hexdigest()
        for item in json.loads(raw.decode("utf-8-sig")).get("observations", []):
            count = item.get("count", item.get("result_count"))
            utc = item.get("observed_at_utc")
            query = item.get("query") or item.get("exact_query") or item.get("submitted_query")
            valid = (isinstance(count, int) and not isinstance(count, bool) and count >= 0
                     and bool(utc) and bool(query) and item.get("status") in ACCEPTED)
            row = dict(field=item.get("field"), trial_id=item.get("trial_id"), source_id=item.get("source_id"),
                       count=count, status=item.get("status"), eligible_for_sum=valid, query=query,
                       field_scope=item.get("field_scope"), source_scope=item.get("source_scope") or item.get("source_collection") or item.get("collection"),
                       filters=item.get("filters") or item.get("filters_and_expanders") or item.get("dates_filter_expander_settings"),
                       expanders=item.get("expanders"),
                       observed_at_utc=utc, observed_at_eastern=eastern(utc), evidence_file=name)
            row["settings_revision"] = item.get("settings_revision")
            row["settings_verification"] = "See source-native settings evidence"
            row["original_expander_assertion"] = item.get("expanders")
            if name == "ebsco-expanded-counts.json":
                explicit = item.get("status") == "observed_explicitly_applied"
                row["settings_verification"] = ("Explicit removal and Apply observed" if explicit else
                    "Prior applied expander state unknown; URL-only assertion corrected by ebsco-expander-audit.json")
                if not explicit:
                    row["expanders"] = "Unknown applied state; original assertion retained separately"
            observations.append(row)
            if valid:
                grouped[row["field"], row["trial_id"], row["source_id"]].append(row)
    summaries = []
    panels = []
    for field in plan["fields"]:
        for source in field["source_ids"]:
            panels.append(dict(field=field["field"], field_role=field["role"], source_id=source,
                               topic=field["topic"], ebsco_supplement_note=field.get("ebsco_supplement_note")))
        for trial in field["trials"]:
            chosen, missing, repeats = {}, [], {}
            for source in field["source_ids"]:
                candidates = grouped[field["field"], trial["trial_id"], source]
                if not candidates:
                    missing.append(source)
                    continue
                chosen[source] = sorted(candidates, key=lambda row: row["observed_at_utc"])[-1]
                if len(candidates) > 1:
                    repeats[source] = [row["count"] for row in candidates]
            subtotal = sum(row["count"] for row in chosen.values())
            complete = not missing
            summaries.append(dict(field=field["field"], trial_id=trial["trial_id"],
                                  expected_sources=len(field["source_ids"]), observed_sources=len(chosen),
                                  observed_subtotal=subtotal if chosen else None,
                                  complete_raw_sum=subtotal if complete else None,
                                  in_target=10000 <= subtotal <= 20000 if complete else None,
                                  missing_sources=missing, repeat_counts=repeats,
                                  source_counts={source: row["count"] for source, row in chosen.items()},
                                  status="all_source_counts_observed" if complete else "partial_source_counts",
                                  corpus_admitted=False))
    result = dict(protocol_id=plan["protocol_id"], generated_at_utc=datetime.now(timezone.utc).isoformat(),
                  scope="Observed counts only; source semantics, final query approval and received exports remain separate gates",
                  input_hashes=hashes, trials=summaries)
    args.public_output.mkdir(parents=True, exist_ok=True)
    write_csv(args.public_output / "expanded-source-panels.csv", list(panels[0]), panels)
    revisions = [dict(field=f["field"], trial_id=t["trial_id"], concept_blocks=t["concept_blocks"],
                      authorized_at_utc=t.get("decision_at_utc", plan["created_at_utc"]),
                      rationale=t.get("rationale", "Initial broad topic query"))
                 for f in plan["fields"] for t in f["trials"]]
    write_csv(args.public_output / "query-revision-plan.csv", list(revisions[0]), revisions)
    amendments = plan.get("source_translation_amendments", [])
    if amendments:
        columns = list(dict.fromkeys(key for row in amendments for key in row))
        write_csv(args.public_output / "source-translation-amendments.csv", columns, amendments)
    if observations:
        write_csv(args.public_output / "expanded-search-observations.csv", list(observations[0]), observations)
    (args.public_output / "expanded-count-status.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    (args.collection / "evidence" / "expanded-count-status.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    for row in summaries:
        print(f"{row['trial_id']}: {row['observed_sources']}/{row['expected_sources']} sources; "
              f"subtotal={row['observed_subtotal']}; full={row['complete_raw_sum']}; in_target={row['in_target']}")


if __name__ == "__main__":
    main()
