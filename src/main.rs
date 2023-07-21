// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use crate::cli::Cli;
use crate::config::{
    collect_config_files, sanitize_config, ConfigId, FromYaml, FromYamlError, MergeWith,
    PortForwardConfig, RetryDelay,
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

    // Attempt to find the configuration file in parent directories and ensure configuration can be loaded.
    let mut configs = Vec::new();

    for (source, file) in collect_config_files(cli.config)? {
        // TODO: Allow skipping of incompatible version (--ignore-errors?)
        let config = match file.into_configuration(&source) {
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
        // TODO: Allow skipping of incompatible version (--ignore-errors?)
        if !config.is_supported_version() {
            eprintln!(
                "Configuration version {loaded} is not supported by this application",
                loaded = config.version
            );
            return exitcode(exitcode::CONFIG);
        }

        configs.push((source, config));
    }

    let mut config = match configs.len() {
        0 => {
            eprintln!("No valid configuration files found");
            return exitcode(exitcode::UNAVAILABLE);
        }
        1 => {
            let (source, config) = configs.into_iter().next().expect("one entry exists");
            println!("Using config from {path}", path = source.path.display());
            config
        }
        n => {
            if cli.verbose {
                println!("Merging configs from {n} locations:");
                for (config, _) in &configs {
                    println!(
                        "- {path}{mode}",
                        path = config.path.display(),
                        mode = if config.auto_detected {
                            " (auto-detected)"
                        } else {
                            ""
                        }
                    );
                }
            } else {
                println!("Merging configs from {n} locations");
            }

            let (_, mut merged) = configs.pop().expect("there is at least one config");
            while let Some((_path, config)) = configs.pop() {
                merged.merge_with(&config);
            }
            merged
        }
    };

    println!();

    // Early exit.
    if config.targets.is_empty() {
        eprintln!("No targets configured.");
        return exitcode(exitcode::CONFIG);
    }

    // Create channels for communication.
    let (out_tx, out_rx) = mpsc::channel();
    let print_thread = start_output_loop_thread(out_rx);

    // Sanitize default values.
    let current_context = kubectl.current_context()?;
    let current_cluster = kubectl.current_cluster()?;

    sanitize_config(&mut config, current_context, current_cluster, &kubectl);

    let operational = config.config.expect("operational config exists");

    // Map out the config.
    println!("Forwarding to the following targets:");
    let map = map_and_print_config(config.targets, cli.tags, cli.verbose);
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
    verbose: bool,
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

        // Print the currently targeted cluster.
        if verbose {
            if let Some(source_file) = &config.source_file {
                println!(
                    "{padding} source:  {source_file}",
                    source_file = source_file.display()
                );
            }
        }

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
