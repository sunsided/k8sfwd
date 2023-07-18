// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use crate::config::RetryDelay;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OperationalConfig {
    /// The number of seconds to delay retries for.
    #[serde(default)]
    pub retry_delay_sec: RetryDelay,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operational_default() {
        let config = serde_yaml::from_str::<OperationalConfig>("").expect("configuration is valid");
        assert_eq!(config.retry_delay_sec, RetryDelay::from_secs(5.0));
    }

    #[test]
    fn test_operational() {
        let config = serde_yaml::from_str::<OperationalConfig>(r#"retry_delay_sec: 3.14"#)
            .expect("configuration is valid");
        assert_eq!(config.retry_delay_sec, RetryDelay::from_secs(3.14))
    }
}
