// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use crate::config::Tag;
use clap::Parser;
use std::fs::File;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Sets a custom config file to load instead of .k8sfwd.
    #[arg(short = 'f', long = "file", value_name = "FILE", value_parser = config_file_exists)]
    pub config: Option<PathBuf>,

    /// Sets a custom path to the kubectl binary.
    #[arg(
        long,
        value_name = "FILE",
        env = "KUBECTL_PATH",
        default_value = "kubectl"
    )]
    pub kubectl: PathBuf,

    /// Specifies the tags of the targets to forward to.
    #[arg(short, long, value_name = "TAGS", num_args = 1.., value_delimiter = ' ', allow_hyphen_values = false)]
    pub tags: Vec<Tag>,
}

fn config_file_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if let Ok(_) = File::open(&path) {
        Ok(path)
    } else {
        Err(format!(
            "The config file `{s}` does not exist or is not a valid file"
        ))
    }
}
