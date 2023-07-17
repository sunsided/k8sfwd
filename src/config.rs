// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

mod operational_config;
mod port;
mod port_forward_config;
mod port_forward_configs;
mod resource_type;
mod retry_delay;
mod tag;

use lazy_static::lazy_static;
use semver::Version;

pub use operational_config::OperationalConfig;
pub use port::Port;
pub use port_forward_config::PortForwardConfig;
pub use port_forward_configs::{FromYaml, FromYamlError, PortForwardConfigs};
pub use resource_type::ResourceType;
pub use retry_delay::RetryDelay;
pub use tag::Tag;

lazy_static! {
    pub static ref LOWEST_SUPPORTED_VERSION: Version = Version::new(0, 1, 0);
    pub static ref HIGHEST_SUPPORTED_VERSION: Version = Version::new(0, 1, 0);
}

pub static DEFAULT_CONFIG_FILE: &'static str = ".k8sfwd";
