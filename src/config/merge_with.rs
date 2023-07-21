// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use std::collections::HashSet;
use std::hash::Hash;

/// Trait for merging configuration instances.
pub trait MergeWith<T = Self> {
    /// Merges the current configuration with the specified other instance.
    fn merge_with(&mut self, other: &T);
}

impl<T> MergeWith<T> for Option<T>
where
    T: Clone,
{
    fn merge_with(&mut self, other: &T) {
        if self.is_some() {
            return;
        }

        *self = Some(other.clone());
    }
}

impl<T> MergeWith<Option<T>> for Option<T>
where
    T: Clone,
{
    fn merge_with(&mut self, other: &Option<T>) {
        if self.is_some() {
            return;
        }

        *self = other.clone();
    }
}

impl<T> MergeWith for HashSet<T>
where
    T: Clone + Hash + Eq,
{
    fn merge_with(&mut self, other: &Self) {
        for other in other {
            self.insert(other.clone());
        }
    }
}
