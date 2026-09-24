#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    pub id: String,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Match {
    pub id: String,
    pub text: String,
    pub score: usize,
}
#[derive(Clone, Debug, Default)]
pub struct LocalRetriever {
    documents: Vec<Document>,
}
impl LocalRetriever {
    pub fn new(documents: Vec<Document>) -> Self {
        Self { documents }
    }
    pub fn search(&self, query: &str, limit: usize) -> Vec<Match> {
        let terms = tokens(query);
        let mut matches: Vec<_> = self
            .documents
            .iter()
            .map(|d| Match {
                id: d.id.clone(),
                text: d.text.clone(),
                score: tokens(&d.text)
                    .into_iter()
                    .filter(|t| terms.contains(t))
                    .count(),
            })
            .filter(|m| m.score > 0)
            .collect();
        matches.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
        matches.truncate(limit);
        matches
    }
}
fn tokens(text: &str) -> std::collections::BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase())
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministically_returns_highest_overlap_first() {
        let r = LocalRetriever::new(vec![
            Document {
                id: "b".into(),
                text: "Rust async channels".into(),
            },
            Document {
                id: "a".into(),
                text: "Rust async tools channels".into(),
            },
        ]);
        assert_eq!(
            r.search("rust async channels", 2)
                .into_iter()
                .map(|m| m.id)
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }
}
