// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use crate::config::PortForwardConfig;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::convert::Infallible;
use std::str::FromStr;

/// A filter for selecting a target.
#[derive(Debug, Clone)]
pub struct TargetFilter {
    filter: String,
}

impl TargetFilter {
    pub fn is_empty(&self) -> bool {
        self.filter.is_empty()
    }
}

pub trait MatchesAnyFilter {
    fn matches(&self, config: &PortForwardConfig) -> bool;
}

impl MatchesAnyFilter for TargetFilter {
    fn matches(&self, config: &PortForwardConfig) -> bool {
        if self.is_empty() {
            return true;
        }

        let filter = self.filter.to_ascii_lowercase();

        if config.target.to_ascii_lowercase().starts_with(&filter) {
            return true;
        }

        // TODO: Add alias property

        if let Some(name) = &config.name {
            if name.to_ascii_lowercase().starts_with(&filter) {
                return true;
            }
        }

        false
    }
}

impl<T> MatchesAnyFilter for T
where
    T: AsRef<[TargetFilter]>,
{
    fn matches(&self, config: &PortForwardConfig) -> bool {
        let this = self.as_ref();
        if this.is_empty() {
            return true;
        }

        for filter in this {
            if filter.matches(config) {
                return true;
            }
        }

        false
    }
}

impl PartialEq for TargetFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
    }
}

impl FromStr for TargetFilter {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { filter: s.into() })
    }
}

impl<'de> Deserialize<'de> for TargetFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let filter = String::deserialize(deserializer)?;
        Ok(Self { filter })
    }
}

impl Serialize for TargetFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.filter)
    }
}
