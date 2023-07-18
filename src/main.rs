// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use crate::cli::Cli;
use crate::config::{
    collect_config_files, sanitize_config, ConfigId, FromYaml, FromYamlError, PortForwardConfig,
    RetryDelay,
};
use crate::kubectl::{ChildEvent, Kubectl, RestartPolicy, StreamSource};
use anyhow::Result;
use clap::Parser;
use just_a_tag::{MatchesAnyTagUnion, TagUnion};
use std::collections::HashMap;
use std::process::ExitCode;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use std::{env, thread};

mod banner;
mod cli;
mod config;
mod kubectl;

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

    print_header(kubectl_version);

    // TODO: Watch the configuration file, stop missing bits and start new ones. (Hash the entries?)

    // Attempt to find the configuration file in parent directories.
    let files = collect_config_files(cli.config)?;
    let num_configs = files.len();

    // TODO: Merge configuration from different files.
    // TODO: Merge configuration files' "targets" sections by name (topmost entry wins), otherwise append.
    // TODO: load and sanitize each configuration file
    let (path, file) = files.into_iter().next().expect("at least one file exists");

    if num_configs == 1 {
        println!("Using config from {path}", path = path.display());
    } else {
        println!("Using config from {num_configs} locations");
        // TODO: Print all sources when --verbose is used
    }

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

    // Early exit.
    if configs.targets.is_empty() {
        eprintln!("No targets configured.");
        return exitcode(exitcode::CONFIG);
    }

    // Create channels for communication.
    let (out_tx, out_rx) = mpsc::channel();
    let print_thread = start_output_loop_thread(out_rx);

    // Sanitize default values.
    let current_context = kubectl.current_context()?;
    let current_cluster = kubectl.current_cluster()?;

    sanitize_config(&mut configs, current_context, current_cluster, &kubectl);

    let operational = configs.config.expect("operational config exists");

    // Map out the config.
    println!("Forwarding to the following targets:");
    let map = map_and_print_config(configs.targets, cli.tags);
    if map.is_empty() {
        eprintln!("No targets selected.");
        return exitcode(exitcode::OK);
    }
    println!();

    // For each configuration, attempt a port-forward.
    println!("Spawning child processes:");
    let mut handles = Vec::new();
    for (id, fwd_config) in map {
        // TODO: Fail all or fail some?
        let handle =
            kubectl.port_forward(id, operational.clone(), fwd_config.clone(), out_tx.clone())?;
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap_or(Ok(()))?;
    }

    print_thread.join().ok();

    exitcode(exitcode::OK)
}

fn print_header(kubectl_version: String) {
    banner::Banner::println();
    println!(
        "k8s:fwd {} - a Kubernetes multi-cluster port forwarder",
        env!("CARGO_PKG_VERSION")
    );
    println!("Using kubectl version {kubectl_version}");
}

/// Prints out the details about the current configuration.
///
/// This method also unifies the "current" context/cluster configuration with the
/// actual values previously read from kubectl.
fn map_and_print_config(
    configs: Vec<PortForwardConfig>,
    tags: Vec<TagUnion>,
) -> HashMap<ConfigId, PortForwardConfig> {
    let mut map: HashMap<ConfigId, PortForwardConfig> = HashMap::new();
    for (id, config) in configs.into_iter().enumerate() {
        if !tags.is_empty() && !tags.matches_set(&config.tags) {
            continue;
        }

        let id = ConfigId::new(id);
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
        println!(
            "{padding} context: {}",
            config.context.as_deref().unwrap_or("(implicit)")
        );

        // Print the currently targeted cluster
        println!(
            "{padding} cluster: {}",
            config.cluster.as_deref().unwrap_or("(implicit)")
        );

        map.insert(id, config);
    }
    map
}

fn start_output_loop_thread(out_rx: Receiver<ChildEvent>) -> JoinHandle<()> {
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
                    eprintln!("{id}: An error occurred: {}", error);
                }
            }
        }
    });
    print_thread
}

fn exitcode(code: exitcode::ExitCode) -> Result<ExitCode, anyhow::Error> {
    debug_assert!(code <= u8::MAX as i32);
    Ok(ExitCode::from(code as u8))
}
