use crate::config::{OperationalConfig, PortForwardConfig, RetryDelay};
use crate::ConfigId;
use serde::Deserialize;
use std::env::current_dir;
use std::io::{BufRead, Read};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::{io, thread};

#[derive(Debug)]
pub struct Kubectl {
    kubectl: PathBuf,
    current_dir: PathBuf,
}

impl Kubectl {
    pub fn new(kubectl: PathBuf) -> Result<Self, ShellError> {
        Ok(Self {
            kubectl,
            current_dir: current_dir()?,
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

    pub fn port_forward(
        &self,
        id: ConfigId,
        config: OperationalConfig,
        fwd_config: PortForwardConfig,
        out_tx: Sender<ChildEvent>,
    ) -> Result<JoinHandle<Result<(), anyhow::Error>>, VersionError> {
        let target = format!(
            "{resource}/{name}",
            resource = fwd_config.r#type.to_arg(),
            name = fwd_config.target
        );

        let kubectl = self.kubectl.clone();
        let current_dir = self.current_dir.clone();

        let child_thread = thread::spawn(move || {
            let mut bootstrap = true;
            'new_process: loop {
                // Only delay start at the second iteration.
                if !bootstrap && config.retry_delay_sec > RetryDelay::NONE {
                    thread::sleep(config.retry_delay_sec.into());
                }
                bootstrap = false;

                let mut command = Command::new(kubectl.clone());
                command
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

                // TODO: Handle invalid addresses (e.g. not an IP, not "localhost", ...)
                // TODO: Handle invalid port configurations

                let mut child = command.spawn()?;

                // Read stdout and stderr in separate threads.
                Self::handle_pipe(
                    id,
                    out_tx.clone(),
                    child.stdout.take(),
                    StreamSource::StdOut,
                );

                // TODO: Handle `Error from server (NotFound): pods "foo-78b4c5d554-6z55j" not found")`?
                Self::handle_pipe(
                    id,
                    out_tx.clone(),
                    child.stderr.take(),
                    StreamSource::StdErr,
                );

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
                        RestartPolicy::WillRestartIn(config.retry_delay_sec),
                    ))
                    .ok();
            }
        });

        Ok(child_thread)
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