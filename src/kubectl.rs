// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use crate::cli::KubectlPathBuf;
use crate::config::{ConfigId, OperationalConfig, PortForwardConfig, RetryDelay};
use serde::Deserialize;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::env::current_dir;
use std::io::{BufRead, Read};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::time::Duration;
use std::{io, process, thread};

#[cfg(not(windows))]
const ENV_PATH_SEPARATOR: char = ':';
#[cfg(windows)]
const ENV_PATH_SEPARATOR: char = ';';

#[derive(Debug)]
pub struct Kubectl {
    kubectl: PathBuf,
    current_dir: PathBuf,
}

impl Kubectl {
    pub fn new(kubectl: Option<KubectlPathBuf>) -> Result<Self, ShellError> {
        let kubectl: PathBuf = kubectl.unwrap_or_default().into();
        let path = kubectl
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(current_dir()?);
        Ok(Self {
            kubectl,
            current_dir: path.to_path_buf(),
        })
    }

    pub fn version(&self) -> Result<String, VersionError> {
        let output = Command::new(&self.kubectl)
            .current_dir(&self.current_dir)
            .args(["version", "--output=json"])
            .output()?;

        let value: KubectlVersion = serde_json::from_slice(&output.stdout)?;
        Ok(value.client_version.git_version)
    }

    /// Gets the currently active contexts.
    pub fn current_context(&self) -> Result<String, ContextError> {
        let output = Command::new(&self.kubectl)
            .current_dir(&self.current_dir)
            .args([
                "config",
                "view",
                "--minify",
                "-o",
                "jsonpath='{.current-context}'",
            ])
            .output()?;

        let value = String::from_utf8_lossy(&output.stdout);
        let value = value.trim_matches('\'');
        Ok(value.into())
    }

    /// Gets the currently active contexts' cluster.
    pub fn current_cluster(&self) -> Result<Option<String>, ContextError> {
        let output = Command::new(&self.kubectl)
            .current_dir(&self.current_dir)
            .args([
                "config",
                "view",
                "--minify",
                "-o",
                "jsonpath='{.clusters[0].name}'",
            ])
            .output()?;

        let value = String::from_utf8_lossy(&output.stdout);
        let value = value.trim_matches('\'');
        if !value.is_empty() {
            Ok(Some(value.into()))
        } else {
            Ok(None)
        }
    }

    /// Given the name of the cluster, identifies a context.
    pub fn context_from_cluster(
        &self,
        cluster: Option<&String>,
    ) -> Result<Option<String>, ContextError> {
        if cluster.is_none() {
            return Ok(None);
        }

        let context = cluster.expect("value exists");
        let jsonpath =
            format!("jsonpath='{{$.contexts[?(@.context.cluster==\"{context}\")].name}}'");
        let output = Command::new(&self.kubectl)
            .current_dir(&self.current_dir)
            .args(["config", "view", "--merge=true", "-o", &jsonpath])
            .output()?;

        let value = String::from_utf8_lossy(&output.stdout);
        let value = value.trim_matches('\'');
        // Array values (in case multiple match) are separated by space.
        let values: Vec<_> = value.split(' ').collect();
        if values.len() > 1 {
            return Ok(None);
        }

        let value = values[0];
        if !value.is_empty() {
            Ok(Some(value.into()))
        } else {
            Ok(None)
        }
    }

    /// Given the name of the context, identifies its cluster.
    pub fn cluster_from_context(
        &self,
        context: Option<&String>,
    ) -> Result<Option<String>, ContextError> {
        if context.is_none() {
            return Ok(None);
        }

        let context = context.expect("value exists");
        let jsonpath =
            format!("jsonpath='{{$.contexts[?(@.name==\"{context}\")].context.cluster}}'");
        let output = Command::new(&self.kubectl)
            .current_dir(&self.current_dir)
            .args(["config", "view", "--merge=true", "-o", &jsonpath])
            .output()?;

        let value = String::from_utf8_lossy(&output.stdout);
        let value = value.trim_matches('\'');
        // Array values (in case multiple match) are separated by space.
        let values: Vec<_> = value.split(' ').collect();
        if values.len() > 1 {
            return Ok(None);
        }

        let value = values[0];
        if !value.is_empty() {
            Ok(Some(value.into()))
        } else {
            Ok(None)
        }
    }

    pub fn port_forward(
        &self,
        id: ConfigId,
        config: OperationalConfig,
        fwd_config: PortForwardConfig,
        out_tx: Sender<ChildEvent>,
    ) -> Result<JoinHandle<Result<(), anyhow::Error>>, VersionError> {
        let target = format!(
            "{resource}/{name}",
            resource = fwd_config.r#type.as_arg(),
            name = fwd_config.target
        );

        let kubectl = self.kubectl.clone();
        let current_dir = self.current_dir.clone();

        let child_thread = thread::spawn(move || {
            let retry_delay_sec = config.retry_delay_sec.expect("retry_delay_sec exists");

            let mut bootstrap = true;
            'new_process: loop {
                // Only delay start at the second iteration.
                if !bootstrap && retry_delay_sec > RetryDelay::NONE {
                    thread::sleep(retry_delay_sec.into());
                }
                bootstrap = false;

                let mut command = Command::new(kubectl.clone());
                command
                    .env("PATH", Self::get_env_path(&current_dir))
                    .current_dir(current_dir.clone())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .args(["port-forward"]);

                // the context to use
                if let Some(context) = &fwd_config.context {
                    command.args(["--context", context]);
                }

                // the cluster to use
                if let Some(cluster) = &fwd_config.cluster {
                    command.args(["--cluster", cluster]);
                }

                // which addresses to listen on locally
                match &fwd_config.listen_addrs[..] {
                    [] => {}
                    addresses => {
                        let addresses = addresses.join(",");
                        command.args(["--address", &addresses]);
                    }
                };

                // the namespace to select
                command.args(["-n", &fwd_config.namespace]);

                // pod/name, deployment/name, service/name
                command.arg(target.clone());

                // Apply the port bindings
                for port in &fwd_config.ports {
                    let value = if let Some(local) = port.local {
                        format!("{local}:{remote}", remote = port.remote)
                    } else {
                        format!(":{remote}", remote = port.remote)
                    };

                    command.arg(&value);
                }

                let mut child = command.spawn()?;

                // Read stdout and stderr in separate threads.
                Self::handle_pipe(
                    id,
                    out_tx.clone(),
                    child.stdout.take(),
                    StreamSource::StdOut,
                );

                // TODO: Handle `Error from server (NotFound): pods "foo-78b4c5d554-6z55j" not found")`
                // TODO: Handle `Unable to listen on port 5012: Listeners failed to create with the following errors: [unable to create listener: Error listen tcp4 127.1.0.1:5012: bind: address already in use]`
                Self::handle_pipe(
                    id,
                    out_tx.clone(),
                    child.stderr.take(),
                    StreamSource::StdErr,
                );

                // TODO: Add TCP keepalive for each port!
                let port = fwd_config.ports[0];
                let keepalive = thread::spawn(move || {
                    // TODO: Use fwd_config.listen_addrs to bind.
                    let port = port.local.unwrap_or(port.remote);
                    let mut addrs = format!("127.0.0.1:{port}")
                        .to_socket_addrs()
                        .expect("Failed to parse socket addresses");
                    let addr = addrs.next().expect("Failed to obtain socket address");
                    let addr = SockAddr::from(addr);
                    let stream = match Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
                    {
                        Ok(socket) => {
                            socket.set_nodelay(true).expect("Failed to set TCP_NODELAY");
                            socket
                                .set_keepalive(true)
                                .expect("Failed to set SO_KEEPALIVE");
                            // TODO: stream.set_tcp_keepalive() ?
                            socket
                                .connect(&addr)
                                .expect("Failed to connect to socket address");
                            TcpStream::from(socket)
                        }
                        Err(_e) => {
                            return;
                        }
                    };

                    // TODO: Do something with the stream ... or not.
                    loop {
                        if let Ok(Some(e)) = stream.take_error() {
                            eprintln!("Error on TCP keepalive stream: {e}");
                            return;
                        }
                        thread::sleep(Duration::from_secs(10));
                    }
                });

                let mut child = ChildGuard(child);

                // Wait for the child process to finish
                let status = child.wait();
                let status = match status {
                    Ok(status) => status,
                    Err(e) => {
                        out_tx.send(ChildEvent::Error(id, ChildError::Wait(e))).ok();
                        // TODO: Break out of this loop if the error is unfixable?
                        continue 'new_process;
                    }
                };

                out_tx
                    .send(ChildEvent::Exit(
                        id,
                        status,
                        RestartPolicy::WillRestartIn(retry_delay_sec),
                    ))
                    .ok();
            }
        });

        Ok(child_thread)
    }

    fn get_env_path(current_dir: &Path) -> String {
        let mut path = std::env::var("PATH").unwrap_or_else(|_| String::new());
        if !path.is_empty() {
            path.push(ENV_PATH_SEPARATOR);
        }
        path.push_str(&current_dir.display().to_string());
        path
    }

    fn handle_pipe<T: Read + Send + 'static>(
        id: ConfigId,
        out_tx: Sender<ChildEvent>,
        pipe: Option<T>,
        source: StreamSource,
    ) {
        if let Some(pipe) = pipe {
            thread::spawn(move || {
                let reader = io::BufReader::new(pipe);
                for line in reader.lines() {
                    if line.is_err() {
                        break;
                    }

                    let line = line.unwrap();
                    out_tx.send(ChildEvent::Output(id, source, line)).ok();
                }
            });
        }
    }
}

#[derive(Debug)]
pub enum ChildEvent {
    Output(ConfigId, StreamSource, String),
    Exit(ConfigId, ExitStatus, RestartPolicy),
    Error(ConfigId, ChildError),
}

#[derive(Debug)]
pub enum RestartPolicy {
    WillRestartIn(RetryDelay),
}

#[derive(Debug, thiserror::Error)]
pub enum ChildError {
    /// Failed to wait for the child process' status.
    #[error(transparent)]
    Wait(#[from] io::Error),
}

#[derive(Debug, Copy, Clone)]
pub enum StreamSource {
    StdOut,
    StdErr,
}

#[derive(Deserialize)]
struct KubectlVersion {
    #[serde(alias = "clientVersion")]
    client_version: KubectlClientVersion,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct KubectlClientVersion {
    major: String,
    minor: String,
    #[serde(alias = "gitVersion")]
    git_version: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ShellError {
    #[error(transparent)]
    CommandFailed(#[from] io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum VersionError {
    #[error("The version format could not be read")]
    InvalidFormat(#[from] serde_json::Error),
    #[error(transparent)]
    CommandFailed(#[from] io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error(transparent)]
    CommandFailed(#[from] io::Error),
}

/// A guard to ensure the child process is terminated when the thread is cancelled.
struct ChildGuard(process::Child);

impl ChildGuard {
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        self.0.wait()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.0.kill().ok();
    }
}
