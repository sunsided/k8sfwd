// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use same_file::is_same_file;
use std::path::PathBuf;

/// Tracks directories that were visited during a configuration scan.
#[derive(Debug, Default)]
pub struct VisitTracker {
    visited: Vec<PathBuf>,
}

impl VisitTracker {
    /// Tracks file duplications.
    ///
    /// This differs from [`track_directory`](Self::track_directory) in that
    /// it canonicalizes the file path and registers the owning directory.
    pub fn track_file_path(&mut self, file: &PathBuf) -> Result<bool, std::io::Error> {
        if let Some(directory) = file.canonicalize()?.parent() {
            let path_buf = directory.to_path_buf();
            return self.track_directory(&path_buf);
        }

        Ok(false)
    }

    /// Tracks directory duplications.
    pub fn track_directory(&mut self, dir: &PathBuf) -> Result<bool, std::io::Error> {
        let visited = self.path_already_visited(dir)?;
        if visited {
            return Ok(true);
        }

        self.visited.push(dir.clone().canonicalize()?);
        Ok(false)
    }

    /// Tests whether a path was already visited before.
    fn path_already_visited(&self, test_path: &PathBuf) -> Result<bool, std::io::Error> {
        for path in &self.visited {
            match is_same_file(path, &test_path) {
                Ok(true) => return Ok(true),
                Ok(false) => continue,
                Err(e) => return Err(e),
            }
        }

        Ok(false)
    }
}
