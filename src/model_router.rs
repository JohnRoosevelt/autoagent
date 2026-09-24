#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRoute {
    pub primary: String,
    pub fallbacks: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error("model route must include a primary model")]
    MissingPrimary,
}

pub struct ModelRouter {
    route: ModelRoute,
}

impl ModelRouter {
    pub fn new(route: ModelRoute) -> Result<Self, RouterError> {
        if route.primary.trim().is_empty() {
            return Err(RouterError::MissingPrimary);
        }
        Ok(Self { route })
    }
    /// Ordered candidates let a caller decide whether a concrete provider error permits fallback.
    pub fn candidates(&self) -> Vec<&str> {
        std::iter::once(self.route.primary.as_str())
            .chain(self.route.fallbacks.iter().map(String::as_str))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_primary_before_configured_fallbacks() {
        let router = ModelRouter::new(ModelRoute {
            primary: "fast".into(),
            fallbacks: vec!["reliable".into(), "last".into()],
        })
        .unwrap();
        assert_eq!(router.candidates(), ["fast", "reliable", "last"]);
    }
    #[test]
    fn rejects_empty_primary() {
        assert!(matches!(
            ModelRouter::new(ModelRoute {
                primary: " ".into(),
                fallbacks: vec![]
            }),
            Err(RouterError::MissingPrimary)
        ));
    }
}
