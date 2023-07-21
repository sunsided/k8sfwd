// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct ConfigId(usize);

impl ConfigId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

impl From<usize> for ConfigId {
    fn from(value: usize) -> Self {
        ConfigId::new(value)
    }
}

impl Display for ConfigId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}
