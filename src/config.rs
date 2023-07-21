// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

mod config_id;
mod merge_with;
mod operational_config;
mod port;
mod port_forward_config;
mod port_forward_configs;
mod resource_type;
mod retry_delay;
mod visit_tracker;

use lazy_static::lazy_static;
use semver::Version;
use std::fs::File;
use std::path::PathBuf;
use std::{env, io};

use crate::config::visit_tracker::VisitTracker;
use crate::kubectl::Kubectl;
pub use config_id::ConfigId;
pub use merge_with::MergeWith;
pub use operational_config::OperationalConfig;
pub use port::Port;
pub use port_forward_config::PortForwardConfig;
pub use port_forward_configs::{FromYaml, FromYamlError, PortForwardConfigs};
pub use resource_type::ResourceType;
pub use retry_delay::RetryDelay;

lazy_static! {
    pub static ref LOWEST_SUPPORTED_VERSION: Version = Version::new(0, 1, 0);
    pub static ref HIGHEST_SUPPORTED_VERSION: Version = Version::new(0, 3, 0);
}

pub static DEFAULT_CONFIG_FILE: &'static str = ".k8sfwd";

/// Describes the source and handling of a configuration.
#[derive(Debug)]
pub struct ConfigMeta {
    /// The path to the file.
    pub path: PathBuf,
    /// Whether the path to the file automatically detected (if `true`) or
    /// explicitly specified on the command-line (if `false`).
    pub auto_detected: bool,
    /// Whether only to load the [`OperationalConfig`] from the file
    /// (if `true`, e.g. when automatically detected in presence of an explicitly
    /// specified file), or to load everything (if `false`).
    pub load_config_only: bool,
}

/// This method also unifies the "current" context/cluster configuration with the
/// actual values previously read from kubectl.
pub fn sanitize_config(
    config: &mut PortForwardConfigs,
    current_context: String,
    current_cluster: Option<String>,
    kubectl: &Kubectl,
) {
    if let Some(operational) = &mut config.config {
        operational.sanitize();
    } else {
        config.config = Some(OperationalConfig::default());
    }

    for config in config.targets.iter_mut() {
        autofill_context_and_cluster(config, kubectl, &current_context, &current_cluster);
    }
}

/// Fills the context and cluster name depending on which values are missing.
fn autofill_context_and_cluster(
    config: &mut PortForwardConfig,
    kubectl: &Kubectl,
    current_context: &String,
    current_cluster: &Option<String>,
) {
    match (&mut config.context, &mut config.cluster) {
        (Some(_context), Some(_cluster)) => { /* nothing to do */ }
        (Some(context), None) => match kubectl.cluster_from_context(Some(&context)) {
            Ok(Some(cluster)) => {
                config.cluster = Some(cluster);
            }
            Ok(None) => {}
            Err(_) => {}
        },
        (None, Some(cluster)) => match kubectl.context_from_cluster(Some(&cluster)) {
            Ok(Some(context)) => {
                config.context = Some(context);
            }
            Ok(None) => {}
            Err(_) => {}
        },
        (None, None) => {
            config.context = Some(current_context.clone());
            config.cluster = current_cluster.clone();
        }
    }
}

/// Enumerates all configuration files along the path hierarchy,
/// in the user's home directory and the user's config directory, in that order.
pub fn collect_config_files(
    // TODO: Allow more than file
    cli_file: Vec<PathBuf>,
) -> Result<Vec<(ConfigMeta, File)>, FindConfigFileError> {
    let mut files = Vec::new();
    let mut visited_paths = VisitTracker::default();

    let load_config_only = !cli_file.is_empty();

    // Try file from the CLI arguments.
    for path in cli_file.into_iter() {
        let file = File::open(&path)?;
        // Ensure we don't specify the same file multiple times.
        // We also return any errors since these files are explicitly specified.
        if !visited_paths.track_file_path(&path)? {
            // TODO: Attach file name to the error
            files.push((
                ConfigMeta {
                    path,
                    auto_detected: false,
                    load_config_only: false,
                },
                file,
            ));
        }
    }

    // Look for config file in current_dir + it's parents -> $HOME -> $HOME/.config
    let config = PathBuf::from(DEFAULT_CONFIG_FILE);
    let working_dir = env::current_dir()?;

    let mut current_dir = working_dir.clone();
    let mut levels_deep = 0;
    loop {
        levels_deep += 1;
        // Ignore the path if it was already specified by explicit arguments.
        if let Ok(false) = visited_paths.track_directory(&current_dir) {
            let path = current_dir.join(&config);
            if let Ok(file) = File::open(&path) {
                // Provide an easier to read path by keeping it relative if we
                // are close to the current working directory.
                let path = if levels_deep <= 4 {
                    pathdiff::diff_paths(&path, &working_dir).unwrap_or(path)
                } else {
                    path.canonicalize()?
                };

                files.push((
                    ConfigMeta {
                        path,
                        auto_detected: true,
                        load_config_only,
                    },
                    file,
                ));
            } else {
                // TODO: Log error about invalid file
            }
        }

        if let Some(parent) = current_dir.parent() {
            current_dir = PathBuf::from(parent);
        } else {
            break;
        }
    }

    // $HOME
    if let Some(home_dir_path) = dirs::home_dir() {
        if let Ok(false) = visited_paths.track_directory(&home_dir_path) {
            let path = home_dir_path.join(&config);
            if let Ok(file) = File::open(&path) {
                files.push((
                    ConfigMeta {
                        path,
                        auto_detected: true,
                        load_config_only,
                    },
                    file,
                ));
            } else {
                // TODO: Log error about invalid file
            }
        }
    }

    // On Linux this will be $XDG_CONFIG_HOME
    // Or just $HOME/.config if the above is not present
    if let Some(config_dir_path) = dirs::config_dir() {
        if let Ok(false) = visited_paths.track_directory(&config_dir_path) {
            let path = config_dir_path.join(&config);
            if let Ok(file) = File::open(&path) {
                files.push((
                    ConfigMeta {
                        path,
                        auto_detected: true,
                        load_config_only,
                    },
                    file,
                ));
            } else {
                // TODO: Log error about invalid file
            }
        }
    }

    if files.is_empty() {
        Err(FindConfigFileError::FileNotFound)
    } else {
        Ok(files)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FindConfigFileError {
    #[error("No config file could be found in the path hierarchy")]
    FileNotFound,
    #[error(transparent)]
    InvalidWorkingDirectory(#[from] io::Error),
}
