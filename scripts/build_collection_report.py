"""Publish allowlisted collection metadata from the private, dated evidence files.

Does not publish RIS contents, mailbox identifiers, signed links or browser logs.
This is an evidence index, not a deduplication or model evaluation.
"""

import argparse
import csv
from collections import Counter, defaultdict
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
from zoneinfo import ZoneInfo


FIELDS = ["MED", "NEU", "PH", "PSY", "PHA", "ENG", "HS", "ENV", "EDU", "CS", "SPED"]


def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def text(value):
    if value is None:
        return ""
    if isinstance(value, (list, dict)):
        return json.dumps(value, ensure_ascii=False, sort_keys=True)
    return str(value)


def eastern(utc):
    if not utc:
        return ""
    stamps = re.findall(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?(?:Z|[+-]\d\d:\d\d)", utc)
    if len(stamps) == 2:
        return " / ".join(eastern(stamp) for stamp in stamps)
    return datetime.fromisoformat(utc.replace("Z", "+00:00")).astimezone(
        ZoneInfo("America/New_York")
    ).isoformat()


def doi(value):
    return (value or "").removeprefix("https://doi.org/").lower().strip()


def source_name(value):
    name = value.strip()
    aliases = {
        "psycinfo": "APA PsycInfo", "apa psycinfo": "APA PsycInfo",
        "psycarticles": "APA PsycArticles", "apa psycarticles": "APA PsycArticles",
        "cinahl": "CINAHL (edition unspecified)",
        "web of science": "Web of Science (indexes unspecified)",
        "cochrane library": "Cochrane Library (constituent unspecified)",
        "springer link": "SpringerLink", "scopus": "Scopus",
        "cochrane central register of controlled trials": "CENTRAL",
        "world health organization's international clinical trials registry platform": "WHO ICTRP",
    }
    return aliases.get(name.casefold().replace("\u2019", "'"), name)


def source_kind(name, original):
    if name == "WHO ICTRP":
        return "trial_registry"
    if name in {"Web of Science (indexes unspecified)",
                "Cochrane Library (constituent unspecified)", "EBSCOhost", "ProQuest", "Ovid"}:
        return "platform_or_unspecified_collection"
    return {"bibliographic_index": "bibliographic_database",
            "platform_unspecified_indexes": "platform_or_unspecified_collection"}.get(original, original)


def write_csv(path, columns, rows):
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns, extrasaction="raise")
        writer.writeheader()
        writer.writerows({k: text(row.get(k)) for k in columns} for row in rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("collection", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    base = args.collection / "evidence"
    inputs = {name: base / f"{name}-research.json" for name in
              ("biomedical", "physical", "education-psych")}
    input_bytes = {name: path.read_bytes() for name, path in inputs.items()}
    data = {name: json.loads(raw.decode("utf-8-sig")) for name, raw in input_bytes.items()}
    reviews, uses, ranks, papers = [], [], [], []

    def review(field, record, ident, sources, url, locator, observed, status):
        assert ident and doi(record.get("doi")) and url and locator and observed, (field, ident)
        reviews.append(dict(field=field, review_id=ident, doi=doi(record["doi"]),
                            title=record["title"], journal=record.get("journal"),
                            year=record.get("year"), methods_url=url, methods_locator=locator,
                            observed_at_utc=observed, observed_at_eastern=eastern(observed),
                            verification_status=status))
        for raw, normalized, kind, conflict in sources:
            normalized = source_name(normalized)
            uses.append(dict(field=field, review_id=ident, reported_source=raw,
                             normalized_source=normalized, source_type=source_kind(normalized, kind),
                             internally_conflicting=conflict))

    def paper(field, journal, record, metric, value, metric_url, observed, selection,
              inspected, inspection_url, inspection_time, keywords, *,
              selection_note=None, metric_effective_date=None, additional_metrics=None,
              seed=None, field_fit=None, exclusion_reason=None, metric_unit=None, source_date_note=None):
        papers.append(dict(field=field, journal=journal, doi=doi(record.get("doi")),
                           title=record.get("title"), publication_date=record.get("publication_date"),
                           reported_year=record.get("year") or record.get("publication_year"),
                           first_online_date=record.get("first_online_date"), issue_year=record.get("issue_year"),
                           print_date=record.get("print_date"),
                           date_window_note=record.get("date_window_note") or record.get("year_note"),
                           source_date_note=source_date_note,
                           metric=metric, metric_unit=metric_unit, metric_value=value, metric_source_url=metric_url,
                           metric_observed_at_utc=observed, metric_effective_date=metric_effective_date,
                           additional_metrics=additional_metrics, selection=selection, selection_note=selection_note,
                           inspection_status=inspected, inspection_url=inspection_url,
                           inspection_observed_at_utc=inspection_time, keywords=keywords,
                           used_as_keyword_seed=seed, field_fit=field_fit, keyword_exclusion_reason=exclusion_reason))

    def crossref_papers(code, selections):
        def date_parts(record, key):
            parts = (record.get(key) or {}).get("date-parts", [])
            return "-".join(str(n) if i == 0 else f"{n:02d}" for i, n in enumerate(parts[0])) if parts else None

        for p in selections:
            selected = p.get("selected_record") or p.get("selected_paper")
            if not selected:
                continue
            record = dict(doi=selected["DOI"], title="; ".join(selected["title"]),
                          publication_date=date_parts(selected, "published"),
                          first_online_date=date_parts(selected, "published-online"),
                          issue_year=(date_parts(selected, "published-print") or "")[:4],
                          print_date=date_parts(selected, "published-print"),
                          date_window_note="Provider published-date filter; print and online date precision preserved.")
            ins = p.get("manuscript_inspection") or p.get("article_evidence") or {}
            paper(code, p["journal"], record, "Crossref is-referenced-by-count", selected["is-referenced-by-count"],
                  p["query_url"], p["observed_at_utc"], p.get("selection_frame") or p.get("selection_rule"),
                  p.get("status"), ins.get("url") or ins.get("inspected_url") or ins.get("observed_url"),
                  ins.get("observed_at_utc") or ins.get("visited_at_utc"), ins.get("keywords") or ins.get("terms"),
                  selection_note=p.get("comparability_note") or p.get("frame_difference_note"),
                  metric_unit="citing Crossref works", source_date_note=ins.get("date_note"))

    for field in data["biomedical"]["fields"]:
        code = field["code"]
        for r in field["verified_reviews"]:
            sources = [(s["reported_name"], s["normalized_name"], s["source_type"], False)
                       for s in r["normalized_search_sources"]]
            review(code, r, r["id"], sources, r.get("url") or r.get("fulltext_url"),
                   r["methods_locator"], r["observed_at_utc"], r["verification_status"])
        for r in field["journal_ranking"]:
            ranks.append(dict(field=code, rank=r["rank"], journal=r["journal"],
                              issn=r.get("issn"), metric=r["metric"], metric_value=r["sjr"],
                              metric_year=r["ranking_year"], category=r["category"],
                              source_url=r.get("source_url") or r.get("source_urls"),
                              observed_at_utc=r.get("observed_at_utc") or r.get("source_observations_utc")))
        for p in field["recent_papers"]:
            ins = p.get("manuscript_inspection") or {}
            paper(code, p["journal"], p, p.get("metric"), p.get("metric_value"),
                  p.get("metric_source_url"), p.get("observed_at_utc"), p.get("popularity_status"),
                  p.get("manuscript_inspection_status"), ins.get("url"), ins.get("observed_at_utc"), p.get("keywords"))
        crossref_papers(code, field.get("crossref_selections", []))

    physical = data["physical"]
    excluded = {doi(p["doi"]): p["reason"] for p in physical["ranked_papers_excluded_from_keyword_recommendations"]}
    for r in physical["reviews"]:
        sources = [(s["raw_name"], s["normalized_name"], s["source_type"], s["internally_conflicting"])
                   for s in r["normalized_search_sources"]]
        review(r["field"], r, r["evidence_id"], sources, r["methods_url"], r["section"],
               r["observed_at_utc"], "fulltext_methods_verified")
    for group in physical["journal_rankings"]:
        for r in group["journals"]:
            ranks.append(dict(field=group["field"], rank=r.get("composite_rank") or r.get("rank"),
                              journal=r["title"], issn=r.get("issn"), metric=group["metric"],
                              metric_value=r["sjr"], metric_year=group["year"], category=group["scope"],
                              source_url=r["source_url"], observed_at_utc=r.get("observed_at_utc") or group.get("source_observed_at_utc")))
    for p in physical["recent_papers"]:
        ins = p.get("manuscript_inspection") or {}
        for code in p["field_memberships"]:
            paper(code, p["journal_title"], p["paper"], "OpenAlex cited_by_count",
                  p["paper"].get("cited_by_count"), p["metric_query_url"], p["observed_at_utc"],
                  p["selection_rule"], p["status"], ins.get("inspected_url"), ins.get("observed_at_utc"), ins.get("keywords"),
                  seed=p.get("used_as_keyword_seed"), field_fit=(ins.get("field_fit") or {}).get(code),
                  exclusion_reason=excluded.get(doi(p["paper"].get("doi"))),
                  source_date_note=ins.get("year_note"), metric_unit="OpenAlex cited_by_count")

    for field in data["education-psych"]["fields"]:
        code = field["field_id"]
        kinds = {source_name(x["name"]): x["source_kind"] for x in field["database_frequencies"]}
        for r in field["verified_reviews"]:
            # A reported label can name a host plus several indexes. Do not zip
            # lists positionally or infer a database from an ambiguous label.
            sources = [(r["exact_database_names"], normalized,
                        kinds.get(source_name(normalized), "unclassified"), False)
                       for normalized in r["normalized_database_names"]]
            review(code, r, r["id"], sources, r["canonical_url"], r["methods_location"],
                   r["visited_at_utc"], r["status"])
        group = field["ranking"]
        journal_rows = [(r, "primary_top10") for r in group["journals"]]
        journal_rows += [(r, "cutoff_tie_extension") for r in group.get("tied_cutoff_extension", [])]
        for r, role in journal_rows:
            ranks.append(dict(field=code, rank=r.get("rank") or r.get("display_position"),
                              rank_basis=r.get("rank_provenance") or "Rank in observed SJR table",
                              display_position=r.get("display_position"), competition_rank=r.get("jif_competition_rank"),
                              ranking_role=role, journal=r["name"],
                              issn=[x for x in [r.get("issn"), r.get("eissn")] if x] or None,
                              metric=group["metric"], metric_value=r["metric_value"], metric_year=group["year"],
                              category=group["category"], source_url=group["source_url"], observed_at_utc=group["visited_at_utc"]))
        for p in field["trend_papers"]:
            metric = p.get("metric") or {}
            paper(code, p["journal"], p, metric.get("provider", p.get("metric_and_selection_note")),
                  metric.get("value", p.get("metric_value")),
                  p.get("selection_source_url") or p.get("article_url"),
                  metric.get("observed_at_utc") or p.get("selection_source_visited_at_utc"), p.get("selection_status"),
                  p.get("inspection_status", "See dated source evidence"), p.get("source_url") or p.get("article_url"),
                  p.get("visited_at_utc"), p.get("keywords"),
                  selection_note=p.get("selection_note") or p.get("selection_method"),
                  metric_effective_date=metric.get("metric_date"), additional_metrics=p.get("additional_metric"),
                  metric_unit=metric.get("unit"))
        for p in field.get("openalex_selections", []):
            ins = p.get("article_evidence") or {}
            paper(code, p["journal"], p["paper"], "OpenAlex cited_by_count", p["paper"]["cited_by_count"],
                  p["query_url"], p["observed_at_utc"], p.get("selection_rule"), "abstract inspected" if ins else "pending",
                  ins.get("url"), ins.get("visited_at_utc"), ins.get("keywords"))
        crossref_papers(code, field.get("crossref_selections", []))

    identities = {}
    for field in data["biomedical"]["fields"]:
        for p in field.get("crossref_selections", []):
            identities[field["code"], p["journal"].casefold()] = (
                p["journal_issns"], p["query_url"], p["observed_at_utc"])
    for field in data["education-psych"]["fields"]:
        for p in field.get("crossref_selections", []):
            identity = p["journal_identity"]
            identities[field["field_id"], p["journal"].casefold()] = (
                identity["all_issns"], identity["journal_metadata_query_url"], identity["journal_observed_at_utc"])
        for p in field.get("openalex_selections", []):
            identities.setdefault((field["field_id"], p["journal"].casefold()), (
                p.get("issns"), p.get("source_query_url") or p["query_url"], p["observed_at_utc"]))
    for rank in ranks:
        rank.setdefault("rank_basis", "Rank in observed SJR table or explicitly labeled union")
        rank.setdefault("display_position", None)
        rank.setdefault("competition_rank", None)
        rank.setdefault("ranking_role", "primary_top10")
        identity = identities.get((rank["field"], rank["journal"].casefold()))
        rank["identity_source_url"] = identity[1] if identity else None
        rank["identity_observed_at_utc"] = identity[2] if identity else None
        if not rank.get("issn") and identity:
            rank["issn"] = identity[0]

    counts = Counter(r["field"] for r in reviews)
    assert counts == Counter({field: 10 for field in FIELDS}), counts
    assert len({r["review_id"] for r in reviews}) == len(reviews), "Review IDs collide"
    assert all(len({r["doi"] for r in reviews if r["field"] == f}) == 10 for f in FIELDS)
    grouped, conflict_ids, types = defaultdict(set), defaultdict(set), defaultdict(set)
    for row in uses:
        key = row["field"], row["normalized_source"]
        grouped[key].add(row["review_id"])
        types[key].add(row["source_type"])
        if row["internally_conflicting"]:
            conflict_ids[key].add(row["review_id"])
    frequencies = [dict(field=f, source=s, source_type="; ".join(sorted(types[f, s])),
                        review_count=len(ids), denominator=counts[f], review_ids=sorted(ids),
                        conflicting_review_ids=sorted(conflict_ids[f, s]),
                        unambiguous_review_count=len(ids - conflict_ids[f, s]))
                   for (f, s), ids in sorted(grouped.items(), key=lambda x: (FIELDS.index(x[0][0]), -len(x[1]), x[0][1]))]
    overlaps = defaultdict(list)
    for r in reviews:
        overlaps[r["doi"]].append(r["review_id"])
    trials, exports = [], []
    for path in sorted(base.glob("*-search-trials.json")):
        log = read(path)
        field = log["field"]
        for trial in log.get("trials", []):
            individual = trial.get("individual") or {}
            if trial.get("raw_sum") is not None:
                assert trial["raw_sum"] == sum(x["count"] for x in individual.values()), trial["id"]
            for db, result in individual.items():
                result = result or {}
                utc = result.get("utc") or result.get("observed_at_utc")
                trials.append(dict(field=field, corpus_role="preserved_ebsco_only_pilot", trial_id=trial["id"], expression=trial["expression"],
                                   rationale=trial.get("rationale"), database_id=db, raw_database_count=result.get("count"),
                                   observed_at_utc=utc, observed_at_eastern=eastern(utc),
                                   raw_sum=trial.get("raw_sum"), combined_displayed_count=(trial.get("combined") or {}).get("count"),
                                   settings=trial.get("settings"), status=trial.get("status")))
        for export in log.get("exports", []):
            utc = export.get("request_utc")
            exports.append(dict(field=field, corpus_role="preserved_ebsco_only_pilot", export_id=export["id"], trial_id=export.get("trial"),
                                database=export.get("database"), database_id=export.get("database_id"),
                                observed_count=export.get("observed_count"), requested_count=export.get("requested_count"),
                                request_utc=utc, request_eastern=eastern(utc), format=export.get("format"),
                                status=export.get("status"), acknowledgment=export.get("acknowledgment")))
    summary = dict(generated_at_utc=datetime.now(timezone.utc).isoformat(),
                   status="collection_in_progress_not_validation", review_entries=len(reviews),
                   unique_review_dois=len(overlaps), reviews_by_field=dict(counts),
                   cross_field_reviews={d: ids for d, ids in overlaps.items() if len(ids) > 1},
                   ranking_rows_by_field=dict(Counter(r["field"] for r in ranks)),
                   topic_evidence_rows_by_field=dict(Counter(p["field"] for p in papers)),
                   topic_count_note="Rows can include alternate papers or multiple metrics; not a completion count.",
                   search_observation_rows=len(trials), export_request_rows=len(exports),
                   private_input_sha256={inputs[name].name: hashlib.sha256(raw).hexdigest() for name, raw in input_bytes.items()})
    args.output.mkdir(parents=True, exist_ok=True)
    tables = {
        "review-sample.csv": (list(reviews[0]), reviews),
        "review-source-use.csv": (list(uses[0]), uses),
        "source-frequencies.csv": (list(frequencies[0]), frequencies),
        "journal-rankings.csv": (list(ranks[0]), ranks),
        "topic-paper-evidence.csv": (list(papers[0]), papers),
        "search-trials.csv": (["field", "corpus_role", "trial_id", "expression", "rationale", "database_id",
                               "raw_database_count", "observed_at_utc", "observed_at_eastern", "raw_sum",
                               "combined_displayed_count", "settings", "status"], trials),
        "export-requests.csv": (["field", "corpus_role", "export_id", "trial_id", "database", "database_id",
                                 "observed_count", "requested_count", "request_utc", "request_eastern",
                                 "format", "status", "acknowledgment"], exports),
    }
    for name, (columns, rows) in tables.items():
        write_csv(args.output / name, columns, rows)
    (args.output / "research-index.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
