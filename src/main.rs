use crate::cli::Cli;
use crate::kubectl::Kubectl;
use crate::portfwd::PortForwardConfigs;
use anyhow::Result;
use clap::Parser;
use semver::Version;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;
use std::{env, io};

mod cli;
mod kubectl;
mod portfwd;

static BANNER: &'static str = indoc::indoc!(
    r#"
    ██╗░░██╗░█████╗░░██████╗░░░░░███████╗██╗░░░░░░░██╗██████╗
    ██║░██╔╝██╔══██╗██╔════╝░██╗░██╔════╝██║░░██╗░░██║██╔══██╗
    █████═╝░╚█████╔╝╚█████╗░░╚═╝░█████╗░░╚██╗████╗██╔╝██║░░██║
    ██╔═██╗░██╔══██╗░╚═══██╗░██╗░██╔══╝░░░████╔═████║░██║░░██║
    ██║░╚██╗╚█████╔╝██████╔╝░╚═╝░██║░░░░░░╚██╔╝░╚██╔╝░██████╔╝
    ╚═╝░░╚═╝░╚════╝░╚═════╝░░░░░░╚═╝░░░░░░░╚═╝░░░╚═╝░░╚═════╝"#
);

fn main() -> Result<ExitCode> {
    let lowest_supported_version = Version::new(0, 1, 0);
    let highest_supported_version = lowest_supported_version.clone();

    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    // Ensure kubectl is available.
    let kubectl = Kubectl::new(cli.kubectl)?;
    let kubectl_version = match kubectl.version() {
        Ok(version) => version,
        Err(e) => {
            eprintln!("Failed to locate kubectl: {e}");
            return exitcode(exitcode::UNAVAILABLE);
        }
    };

    // Print the banner.
    println!("{}", BANNER.trim_start());
    println!("k8s:fwd {}", env!("CARGO_PKG_VERSION"));
    println!("Using kubectl version {kubectl_version}");

    // TODO: Allow to specify a configuration as a command-line argument.
    // TODO: Watch the configuration file, stop missing bits and start new ones. (Hash the entries?)

    // TODO: Add home directory config. See "home" crate. Allow merging of configuration.

    // Attempt to find the configuration file in parent directories.
    let (path, mut file) = match cli.config {
        None => find_config_file()?,
        Some(file) => (file.clone(), File::open(file)?),
    };

    println!("Using config from {path}", path = path.display());
    println!();

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let configs: PortForwardConfigs = serde_yaml::from_str(&contents)?;

    #[allow(clippy::absurd_extreme_comparisons)]
    if configs.version < lowest_supported_version || configs.version > highest_supported_version {
        eprintln!(
            "Configuration version {loaded} is not supported by this application",
            loaded = configs.version
        );
        return exitcode(exitcode::CONFIG);
    }

    // TODO: If no config exists, exit.

    // For each configuration, attempt a port-forward.
    let mut handles = Vec::new();
    for config in configs {
        // TODO: Fail all or fail some?
        let handle = kubectl.port_forward(&config)?;
        handles.push(handle);
    }

    for handle in handles {
        let res = handle.join().unwrap();
        let _ = res?;
    }

    exitcode(exitcode::OK)
}

fn find_config_file() -> Result<(PathBuf, File), FindConfigFileError> {
    let config = PathBuf::from(".k8sfwd");
    let working_dir = env::current_dir()?;
    let mut current_dir = working_dir.clone();
    loop {
        let path = current_dir.join(&config);
        if let Ok(file) = File::open(&path) {
            let path = pathdiff::diff_paths(&path, working_dir).unwrap_or(path);
            return Ok((path, file));
        }

        if let Some(parent) = current_dir.parent() {
            current_dir = PathBuf::from(parent);
        } else {
            break;
        }
    }

    Err(FindConfigFileError::FileNotFound)
}

fn exitcode(code: exitcode::ExitCode) -> Result<ExitCode, anyhow::Error> {
    debug_assert!(code <= u8::MAX as i32);
    Ok(ExitCode::from(code as u8))
}

#[derive(Debug, thiserror::Error)]
enum FindConfigFileError {
    #[error("No config file could be found in the path hierarchy")]
    FileNotFound,
    #[error(transparent)]
    InvalidWorkingDirectory(#[from] io::Error),
}
