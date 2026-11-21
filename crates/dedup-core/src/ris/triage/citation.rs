use super::{Occurrence, publication_year, title};
use crate::normalize::pagination::Pagination;
use crate::ris::{ReportIdentifiers, Values};

/// What a selected occurrence supplies for the inspection display.
#[derive(Debug, PartialEq, Eq)]
pub enum CitationValue {
    Absent,
    Unresolved,
    Conflicting,
    /// One supported key alongside an unsupported field of the same identifier kind.
    IdentifierWithUnsupportedField(String),
    Known(Vec<String>),
}

impl CitationValue {
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

    /// Display normalized fields independently, including for incomplete citations.
    pub fn citation_fields(&self) -> [(&'static str, CitationValue); 11] {
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
        [
            ("Title", single(&["TI", "T1"], title(self))),
            ("Authors", field(&["AU", "A1"], self.authors())),
            (
                "Year",
                single(&["PY", "Y1"], publication_year(self).map(|y| y.to_string())),
            ),
            ("RIS type", scalar("TY")),
            ("Work type", scalar("M3")),
            ("Venue", single(&["JO", "JF", "T2"], self.journal())),
            ("Volume", scalar("VL")),
            ("Issue", scalar("IS")),
            ("Final pages/article", single(&["SP", "EP"], pages)),
            ("DOI", CitationValue::identifier(ids.doi)),
            ("PMID", CitationValue::identifier(ids.pmid)),
        ]
    }
}
