// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use serde::Deserialize;
use std::fmt::{Display, Formatter};
use std::time::Duration;

#[derive(Deserialize, Debug, Copy, Clone, PartialOrd, PartialEq)]
pub struct RetryDelay(f64);

impl RetryDelay {
    pub const NONE: RetryDelay = RetryDelay(0.0);

    pub fn from_secs(delay: f64) -> Self {
        Self(delay.max(0.0))
    }
}

impl Default for RetryDelay {
    fn default() -> Self {
        RetryDelay::from_secs(5.0)
    }
}

impl From<RetryDelay> for Duration {
    fn from(val: RetryDelay) -> Self {
        Duration::from_secs_f64(val.0)
    }
}

impl Display for RetryDelay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} sec", self.0)
    }
}
