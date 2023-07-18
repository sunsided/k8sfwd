// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use crate::config::RetryDelay;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct OperationalConfig {
    /// The number of seconds to delay retries for.
    pub retry_delay_sec: Option<RetryDelay>,
    // TODO: Add mappings of cluster names; useful for merged hierarchical configs
}

impl Default for OperationalConfig {
    fn default() -> Self {
        Self {
            retry_delay_sec: Some(RetryDelay::default()),
        }
    }
}

impl OperationalConfig {
    /// Ensures that values, if set, are valid (or sanitized such that they are valid).
    pub fn sanitize(&mut self) {
        if self.retry_delay_sec.is_some()
            && self.retry_delay_sec.expect("value exists") < RetryDelay::NONE
        {
            self.retry_delay_sec = Some(RetryDelay::NONE);
        } else {
            self.retry_delay_sec = Some(RetryDelay::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operational_default() {
        let mut config =
            serde_yaml::from_str::<OperationalConfig>("").expect("configuration is valid");
        assert_eq!(config.retry_delay_sec, None);

        config.sanitize();
        assert_eq!(config.retry_delay_sec, Some(RetryDelay::default()));
    }

    #[test]
    fn test_operational() {
        let config = serde_yaml::from_str::<OperationalConfig>(r#"retry_delay_sec: 3.14"#)
            .expect("configuration is valid");
        assert_eq!(config.retry_delay_sec, Some(RetryDelay::from_secs(3.14)))
    }
}
