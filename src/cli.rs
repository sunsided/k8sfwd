// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use crate::target_filter::TargetFilter;
use clap::Parser;
use just_a_tag::TagUnion;
use std::fs::File;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use which::which;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Sets a custom config file to load instead of .k8sfwd.
    #[arg(short = 'f', long = "file", value_name = "FILE", value_parser = config_file_exists)]
    pub config: Vec<PathBuf>,

    /// Specifies the prefixes of the target configurations to select.
    #[arg(value_name = "FILTER", num_args = 1.., value_delimiter = ' ', allow_hyphen_values = false)]
    pub filters: Vec<TargetFilter>,

    /// Specifies the tags of the targets to forward to.
    #[arg(short, long, value_name = "TAGS", num_args = 1.., value_delimiter = ' ', allow_hyphen_values = false)]
    pub tags: Vec<TagUnion>,

    /// Sets a custom path to the kubectl binary.
    #[arg(long, value_name = "FILE", env = "KUBECTL_PATH")]
    pub kubectl: Option<KubectlPathBuf>,

    /// Enables verbose log outputs.
    #[arg(long)]
    pub verbose: bool,
}

fn config_file_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if File::open(&path).is_ok() {
        Ok(path)
    } else {
        Err(format!(
            "The config file `{s}` does not exist or is not a valid file"
        ))
    }
}

#[derive(Debug, Clone)]
pub struct KubectlPathBuf(PathBuf);

impl Default for KubectlPathBuf {
    fn default() -> Self {
        Self(which("kubectl").unwrap_or(PathBuf::from("kubectl")))
    }
}

impl Deref for KubectlPathBuf {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<KubectlPathBuf> for PathBuf {
    fn from(val: KubectlPathBuf) -> Self {
        val.0
    }
}

impl FromStr for KubectlPathBuf {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(PathBuf::from_str(s)?))
    }
}
