#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationCase {
    pub name: String,
    pub input: String,
    pub expected: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationResult {
    pub name: String,
    pub passed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationReport {
    pub results: Vec<EvaluationResult>,
}

impl EvaluationReport {
    pub fn passed(&self) -> usize {
        self.results.iter().filter(|result| result.passed).count()
    }
}

/// Runs repeatable local assertions against an injected implementation.
pub fn evaluate(cases: &[EvaluationCase], run: impl Fn(&str) -> String) -> EvaluationReport {
    EvaluationReport {
        results: cases
            .iter()
            .map(|case| EvaluationResult {
                name: case.name.clone(),
                passed: run(&case.input) == case.expected,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_each_repeatable_case_and_its_outcome() {
        let cases = vec![
            EvaluationCase {
                name: "uppercase".into(),
                input: "ok".into(),
                expected: "OK".into(),
            },
            EvaluationCase {
                name: "failure".into(),
                input: "no".into(),
                expected: "yes".into(),
            },
        ];
        let report = evaluate(&cases, str::to_uppercase);
        assert_eq!(report.passed(), 1);
        assert_eq!(
            report.results[1],
            EvaluationResult {
                name: "failure".into(),
                passed: false
            }
        );
    }
}
