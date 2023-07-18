// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use serde::Deserialize;

/// The type of resource to forward to.
#[derive(Debug, Clone, Copy, Deserialize)]
pub enum ResourceType {
    #[serde(rename = "service")]
    Service,
    #[serde(rename = "deployment")]
    Deployment,
    #[serde(rename = "pod")]
    Pod,
}

impl Default for ResourceType {
    fn default() -> Self {
        Self::Service
    }
}

impl ResourceType {
    pub fn to_arg(&self) -> &'static str {
        match self {
            ResourceType::Service => "service",
            ResourceType::Deployment => "deployment",
            ResourceType::Pod => "pod",
        }
    }
}
