use crate::cli::Cli;
use crate::config::{
    FromYaml, FromYamlError, PortForwardConfig, PortForwardConfigs, RetryDelay, DEFAULT_CONFIG_FILE,
};
use crate::kubectl::{ChildEvent, Kubectl, RestartPolicy, StreamSource};
use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use std::{env, io, thread};

mod cli;
mod config;
mod kubectl;

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

    // TODO: Watch the configuration file, stop missing bits and start new ones. (Hash the entries?)

    // TODO: Add home directory config. See "home" crate. Allow merging of configuration.

    // Attempt to find the configuration file in parent directories.
    let (path, file) = match cli.config {
        None => find_config_file()?,
        Some(file) => (file.clone(), File::open(file)?),
    };

    println!("Using config from {path}", path = path.display());
    println!();

    // Ensure configuration can be loaded.
    let mut configs = match file.into_configuration() {
        Ok(configs) => configs,
        Err(FromYamlError::InvalidConfiguration(e)) => {
            eprintln!("Invalid configuration: {e}");
            return exitcode(exitcode::CONFIG);
        }
        Err(FromYamlError::FileReadFailed(e)) => {
            eprintln!("Failed to read configuration file: {e}");
            return exitcode(exitcode::UNAVAILABLE);
        }
    };

    // Ensure version is supported.
    if !configs.is_supported_version() {
        eprintln!(
            "Configuration version {loaded} is not supported by this application",
            loaded = configs.version
        );
        return exitcode(exitcode::CONFIG);
    }

    // Create channels for communication.
    let (out_tx, out_rx) = mpsc::channel();
    let print_thread = run_output_loop(out_rx);

    // Sanitize default values.
    let current_context = kubectl.current_context()?;
    let current_cluster = kubectl.current_cluster()?;
    sanitize_config(&mut configs, current_context, current_cluster);

    // Map out the config.
    println!("Forwarding to the following targets:");
    let map = map_and_print_config(configs.targets);
    println!();

    // For each configuration, attempt a port-forward.
    println!("Spawning child processes:");
    let mut handles = Vec::new();
    for (id, fwd_config) in map {
        // TODO: Fail all or fail some?
        let handle = kubectl.port_forward(
            id,
            configs.config.clone(),
            fwd_config.clone(),
            out_tx.clone(),
        )?;
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap_or(Ok(()))?;
    }

    print_thread.join().ok();

    exitcode(exitcode::OK)
}

/// This method also unifies the "current" context/cluster configuration with the
/// actual values previously read from kubectl.
fn sanitize_config(
    config: &mut PortForwardConfigs,
    current_context: String,
    current_cluster: Option<String>,
) {
    if config.config.retry_delay_sec < RetryDelay::NONE {
        config.config.retry_delay_sec = RetryDelay::NONE;
    }

    for config in config.targets.iter_mut() {
        if config.context.is_none() {
            config.context = Some(current_context.clone());
        }

        // Print the currently targeted cluster
        if config.cluster.is_none() {
            config.cluster = current_cluster.clone();
        }
    }
}

/// Prints out the details about the current configuration.
///
/// This method also unifies the "current" context/cluster configuration with the
/// actual values previously read from kubectl.
fn map_and_print_config(configs: Vec<PortForwardConfig>) -> HashMap<ConfigId, PortForwardConfig> {
    let mut map: HashMap<ConfigId, PortForwardConfig> = HashMap::new();
    for (id, config) in configs.into_iter().enumerate() {
        let id = ConfigId(id);
        let padding = " ".repeat(id.to_string().len());

        if let Some(name) = &config.name {
            println!("{id} {name}");
            println!(
                "{padding} target:  {resource}/{name}.{namespace}",
                resource = config.r#type.to_arg(),
                name = config.target,
                namespace = config.namespace
            );
        } else {
            println!(
                "{id} target:  {resource}/{name}.{namespace}",
                resource = config.r#type.to_arg(),
                name = config.target,
                namespace = config.namespace
            );
        }

        // Print the currently selected context
        debug_assert!(config.context.is_some());
        println!(
            "{padding} context: {}",
            config.context.as_deref().unwrap_or("(current)")
        );

        // Print the currently targeted cluster
        println!(
            "{padding} cluster: {}",
            config.cluster.as_deref().unwrap_or("(current)")
        );

        map.insert(id, config);
    }
    map
}

fn run_output_loop(out_rx: Receiver<ChildEvent>) -> JoinHandle<()> {
    let print_thread = thread::spawn(move || {
        while let Ok(event) = out_rx.recv() {
            match event {
                ChildEvent::Output(id, channel, message) => {
                    // TODO: use display name
                    match channel {
                        StreamSource::StdOut => println!("{id}: {message}"),
                        StreamSource::StdErr => eprintln!("{id}: {message}"),
                    }
                }
                ChildEvent::Exit(id, status, policy) => {
                    // TODO: use display name
                    match policy {
                        RestartPolicy::WillRestartIn(delay) => {
                            if delay > RetryDelay::NONE {
                                eprintln!(
                                    "{id}: Process exited with {} - will retry in {}",
                                    status, delay
                                );
                            } else {
                                eprintln!(
                                    "{id}: Process exited with {} - retrying immediately",
                                    status
                                );
                            }
                        }
                    }
                }
                ChildEvent::Error(id, error) => {
                    // TODO: use display name
                    eprintln!("{id}: An error occured: {}", error);
                }
            }
        }
    });
    print_thread
}

fn find_config_file() -> Result<(PathBuf, File), FindConfigFileError> {
    let config = PathBuf::from(DEFAULT_CONFIG_FILE);
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

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct ConfigId(usize);

impl Display for ConfigId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}
