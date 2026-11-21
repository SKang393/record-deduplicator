use super::{Field, Occurrence, title, year};
use crate::normalize::pagination::Pagination;
use crate::ris::{ReportIdentifiers, Values};
use std::collections::BTreeSet;

/// What a selected occurrence supplies for the inspection display.
#[derive(Debug, PartialEq, Eq)]
pub enum CitationValue {
    Absent,
    Unresolved,
    Conflicting,
    /// One supported key alongside an unsupported field of the same identifier kind.
    IdentifierWithUnsupportedField(String),
    Known(Vec<String>),
    /// Source evidence to display side by side without an equality verdict.
    AsSupplied(Vec<String>),
}

impl CitationValue {
    fn supplied_fields<'a>(fields: impl Iterator<Item = &'a Field<'a>>) -> Self {
        let values: Vec<_> = fields
            .map(|f| {
                let more = if f.continued {
                    " [continued in original]"
                } else {
                    ""
                };
                format!("{}={}{}", f.tag, f.first_line_value, more)
            })
            .collect();
        if values.is_empty() {
            Self::Absent
        } else {
            Self::AsSupplied(values)
        }
    }

    fn identifier(values: Values) -> Self {
        if values.conflict {
            Self::Conflicting
        } else if let Some(key) = values.key {
            if values.unsupported {
                Self::IdentifierWithUnsupportedField(key)
            } else {
                Self::Known(vec![key])
            }
        } else if values.had_field {
            Self::Unresolved
        } else {
            Self::Absent
        }
    }
}

impl Occurrence<'_> {
    /// Literal title ending for a review prompt. The words can also be captions.
    pub fn protocol_or_results_wording(&self) -> Option<(String, &'static str)> {
        let normalized = title(self)?;
        let (stem, word) = normalized.rsplit_once(' ')?;
        let word = match word.strip_suffix('.').unwrap_or(word) {
            "protocol" => "protocol",
            "results" => "results",
            _ => return None,
        };
        stem.chars()
            .any(char::is_alphanumeric)
            .then(|| (stem.to_owned(), word))
    }

    fn inspection_dates(&self) -> (CitationValue, CitationValue) {
        let fields = self
            .fields
            .iter()
            .filter(|f| matches!(f.tag, "PY" | "Y1" | "DA"));
        let supplied = CitationValue::supplied_fields(fields.clone());
        if supplied == CitationValue::Absent {
            return (CitationValue::Absent, CitationValue::Absent);
        }
        // Keep all supplied years. The tags alone cannot assign a print or online date.
        let years = match fields.map(year).collect::<Option<BTreeSet<_>>>() {
            Some(years) => {
                let values: Vec<_> = years.into_iter().map(|y| y.to_string()).collect();
                if values.len() == 1 {
                    CitationValue::Known(values)
                } else {
                    CitationValue::AsSupplied(values)
                }
            }
            None => CitationValue::Unresolved,
        };
        (years, supplied)
    }

    /// Display normalized citation fields alongside literal source evidence for inspection.
    pub fn citation_fields(&self) -> [(&'static str, CitationValue); 13] {
        let field = |tags: &[&str], values| match values {
            Some(values) => CitationValue::Known(values),
            None if self.fields.iter().any(|f| tags.contains(&f.tag)) => CitationValue::Unresolved,
            None => CitationValue::Absent,
        };
        let single = |tags: &[&str], value: Option<String>| field(tags, value.map(|v| vec![v]));
        let scalar = |tag: &str| single(&[tag], self.scalar(tag).flatten().map(str::to_owned));
        let pages = match self.final_pagination() {
            Some(Pagination::Pages {
                start,
                end: Some(end),
                ..
            }) => Some(format!("{start}-{end}")),
            Some(Pagination::ArticleNumber(n)) => Some(format!("article {n}")),
            _ => None,
        };
        let mut ids = ReportIdentifiers::default();
        ids.observe(self);
        let (years, dates) = self.inspection_dates();
        [
            ("Title", single(&["TI", "T1"], title(self))),
            ("Authors", field(&["AU", "A1"], self.authors())),
            ("Reported years", years),
            ("RIS date fields", dates),
            ("RIS type", scalar("TY")),
            ("Work type", scalar("M3")),
            ("Venue", single(&["JO", "JF", "T2"], self.journal())),
            ("Volume", scalar("VL")),
            ("Issue", scalar("IS")),
            ("Final pages/article", single(&["SP", "EP"], pages)),
            ("DOI", CitationValue::identifier(ids.doi)),
            ("PMID", CitationValue::identifier(ids.pmid)),
            (
                "RIS C2 (source text only, unused for matching)",
                CitationValue::supplied_fields(self.fields.iter().filter(|f| f.tag == "C2")),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::CitationValue::{Absent, AsSupplied, Known, Unresolved};
    use crate::ris::parse;

    #[test]
    fn c2_is_supplied_text_even_when_it_matches_a_pubmed_url() {
        let source = parse(
            "TY  - JOUR\nDB  - Scopus\nC2  - 980000001\nC2  - PMC980000001\n  note\nER  -\n\
             TY  - JOUR\nC2  - 980000001\nUR  - https://pubmed.ncbi.nlm.nih.gov/980000001/\nER  -\n",
        )
        .unwrap();
        assert!(source.review().groups.is_empty());
        let fields = |i: usize| source.occurrences[i].citation_fields();
        let c2 = "RIS C2 (source text only, unused for matching)";
        assert!(fields(0).contains(&("PMID", Absent)));
        assert!(fields(0).contains(&(
            c2,
            AsSupplied(vec![
                "C2=980000001".into(),
                "C2=PMC980000001 [continued in original]".into(),
            ])
        )));
        assert!(fields(1).contains(&("PMID", Known(vec!["980000001".into()]))));
        assert!(fields(1).contains(&(c2, AsSupplied(vec!["C2=980000001".into()]))));
    }

    #[test]
    fn inspection_preserves_reported_date_tags() {
        let record = |fields: &str| {
            let text = format!("TY  - JOUR\n{fields}ER  -\n");
            parse(&text).unwrap().occurrences[0].inspection_dates()
        };
        assert_eq!(
            record("DA  - 2021/12/19/\n"),
            (
                Known(vec!["2021".into()]),
                AsSupplied(vec!["DA=2021/12/19/".into()])
            )
        );
        assert_eq!(
            record("PY  - 2022\nDA  - 2021/12/19/\nY1  - 2022/02/\nY2  - 2025/05/10/\n"),
            (
                AsSupplied(vec!["2021".into(), "2022".into()]),
                AsSupplied(vec![
                    "PY=2022".into(),
                    "DA=2021/12/19/".into(),
                    "Y1=2022/02/".into()
                ]),
            )
        );
        assert_eq!(record("Y2  - 2025/05/10/\n"), (Absent, Absent));
        assert_eq!(
            record("DA  - winter issue\n  date pending\n"),
            (
                Unresolved,
                AsSupplied(vec!["DA=winter issue [continued in original]".into()])
            )
        );
    }
}
