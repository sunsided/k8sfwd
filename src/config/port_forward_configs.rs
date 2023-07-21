// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use crate::config::{
    MergeWith, OperationalConfig, PortForwardConfig, HIGHEST_SUPPORTED_VERSION,
    LOWEST_SUPPORTED_VERSION,
};
use semver::Version;
use serde::Deserialize;
use std::fs::File;
use std::io;
use std::io::Read;

#[derive(Debug, Deserialize)]
pub struct PortForwardConfigs {
    pub version: Version,
    #[serde(default)]
    pub config: Option<OperationalConfig>,
    #[serde(default)]
    pub targets: Vec<PortForwardConfig>,
}

impl MergeWith for PortForwardConfigs {
    fn merge_with(&mut self, other: &Self) {
        self.version = other.version.clone();

        match &mut self.config {
            None => self.config = other.config.clone(),
            Some(config) => config.merge_with(&other.config),
        }

        if self.targets.is_empty() {
            self.targets = other.targets.clone();
        } else {
            self.targets.merge_with(&other.targets);
        }
    }
}

impl Default for PortForwardConfigs {
    fn default() -> Self {
        Self {
            version: Version::new(0, 0, 0),
            config: None,
            targets: Vec::new(),
        }
    }
}

pub trait FromYaml {
    fn into_configuration(self) -> Result<PortForwardConfigs, FromYamlError>;
}

impl FromYaml for File {
    fn into_configuration(mut self) -> Result<PortForwardConfigs, FromYamlError> {
        let mut contents = String::new();
        self.read_to_string(&mut contents)?;
        Ok(serde_yaml::from_str(&contents)?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FromYamlError {
    #[error(transparent)]
    InvalidConfiguration(#[from] serde_yaml::Error),
    #[error(transparent)]
    FileReadFailed(#[from] io::Error),
}

impl IntoIterator for PortForwardConfigs {
    type Item = PortForwardConfig;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.targets.into_iter()
    }
}

impl PortForwardConfigs {
    pub fn is_supported_version(&self) -> bool {
        #[allow(clippy::absurd_extreme_comparisons)]
        if self.version < *LOWEST_SUPPORTED_VERSION || self.version > *HIGHEST_SUPPORTED_VERSION {
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entire_config() {
        let config = r#"
            version: 0.1.0
            config:
              retry_delay_sec: 3.14
            targets:
              - name: Test API (Staging)
                target: foo
                type: service
                namespace: bar
                context: null
                cluster: null
                listen_addrs:
                  - "127.0.0.1"
                ports:
                  - "5012:80"
                  - 8080
              - name: Test API (Production)
                target: foo
                type: service
                namespace: bar
                cluster: production
                listen_addrs:
                  - "127.1.0.1"
                ports:
                  - "5012:80"
        "#;

        let config: PortForwardConfigs = serde_yaml::from_str(config).unwrap();
        assert_eq!(config.targets.len(), 2);
    }
}
