use std::{collections::BTreeMap, fs, path::Path};

/// Non-secret runtime settings. Secrets remain in the process environment and are never persisted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    pub model: String,
    pub base_url: String,
    pub max_steps: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            model: "openrouter/free".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            max_steps: 3,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigurationError {
    #[error("cannot read configuration file: {0}")]
    Read(#[from] std::io::Error),
    #[error("invalid configuration line: {0}")]
    InvalidLine(String),
    #[error("AGENT_MAX_STEPS must be a positive integer")]
    InvalidMaxSteps,
}

impl AppConfig {
    /// Applies defaults, then a simple local `KEY=value` file, then environment values.
    /// Unknown keys are intentionally ignored to permit secret/provider settings alongside these values.
    pub fn load(
        path: Option<&Path>,
        environment: &BTreeMap<String, String>,
    ) -> Result<Self, ConfigurationError> {
        let mut values = BTreeMap::new();
        if let Some(path) = path {
            for line in fs::read_to_string(path)?.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let (key, value) = line
                    .split_once('=')
                    .ok_or_else(|| ConfigurationError::InvalidLine(line.into()))?;
                values.insert(key.trim().into(), value.trim().into());
            }
        }
        for (key, value) in environment {
            values.insert(key.clone(), value.clone());
        }
        let mut config = Self::default();
        if let Some(value) = values.get("AGENT_MODEL") {
            config.model = value.clone();
        }
        if let Some(value) = values.get("AGENT_BASE_URL") {
            config.base_url = value.clone();
        }
        if let Some(value) = values.get("AGENT_MAX_STEPS") {
            config.max_steps = value
                .parse()
                .ok()
                .filter(|value: &usize| *value > 0)
                .ok_or(ConfigurationError::InvalidMaxSteps)?;
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn applies_defaults_file_then_environment() {
        let path = std::env::temp_dir().join(format!(
            "autoagent-config-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, "AGENT_MODEL=file-model\nAGENT_MAX_STEPS=5\n").unwrap();
        let environment = BTreeMap::from([("AGENT_MODEL".into(), "env-model".into())]);
        let config = AppConfig::load(Some(&path), &environment).unwrap();
        fs::remove_file(path).unwrap();
        assert_eq!(config.model, "env-model");
        assert_eq!(config.max_steps, 5);
        assert_eq!(config.base_url, "https://openrouter.ai/api/v1");
    }

    #[test]
    fn rejects_invalid_or_zero_step_count() {
        let environment = BTreeMap::from([("AGENT_MAX_STEPS".into(), "0".into())]);
        assert!(matches!(
            AppConfig::load(None, &environment),
            Err(ConfigurationError::InvalidMaxSteps)
        ));
    }
}
