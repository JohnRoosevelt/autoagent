use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Span {
    pub name: String,
    pub fields: BTreeMap<String, String>,
}

#[derive(Default)]
pub struct TraceRecorder {
    spans: Vec<Span>,
}

impl TraceRecorder {
    pub fn record(&mut self, name: impl Into<String>, fields: BTreeMap<String, String>) {
        self.spans.push(Span {
            name: name.into(),
            fields,
        });
    }
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn records_structured_spans_in_order() {
        let mut trace = TraceRecorder::default();
        trace.record(
            "model.call",
            BTreeMap::from([("model".into(), "local".into())]),
        );
        trace.record("tool.call", BTreeMap::new());
        assert_eq!(
            trace
                .spans()
                .iter()
                .map(|span| span.name.as_str())
                .collect::<Vec<_>>(),
            ["model.call", "tool.call"]
        );
        assert_eq!(trace.spans()[0].fields["model"], "local");
    }
}
