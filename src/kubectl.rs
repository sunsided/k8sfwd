use crate::portfwd::PortForwardConfig;
use serde::Deserialize;
use std::any::type_name;
use std::env::current_dir;
use std::io::{BufRead, Read};
use std::os::linux::raw::stat;
use std::path::PathBuf;
use std::process::{ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::{env, io, thread};

#[derive(Debug)]
pub struct Kubectl {
    kubectl: String,
    current_dir: PathBuf,
}

impl Kubectl {
    pub fn new() -> Result<Self, ShellError> {
        Ok(Self {
            kubectl: env::var("KUBECTL_PATH").unwrap_or("kubectl".to_string()),
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

    pub fn port_forward(
        &self,
        config: &PortForwardConfig,
    ) -> Result<JoinHandle<Result<(), anyhow::Error>>, VersionError> {
        let target = format!(
            "{resource}/{name}",
            resource = config.r#type.to_arg(),
            name = config.target
        );

        let display_name = config.name.to_owned().unwrap_or(format!(
            "{host}.{namespace}",
            host = config.target,
            namespace = config.namespace
        ));

        let mut command = Command::new(&self.kubectl);
        command
            .current_dir(&self.current_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(["port-forward"]);

        // the context to use
        if let Some(context) = &config.context {
            command.args(["--context", context]);
        }

        // the cluster to use
        if let Some(cluster) = &config.cluster {
            command.args(["--cluster", cluster]);
        }

        // which addresses to listen on locally
        match &config.listen_addrs[..] {
            [] => {}
            addresses => {
                let addresses = addresses.join(",");
                command.args(["--address", &addresses]);
            }
        };

        // the namespace to select
        command.args(["-n", &config.namespace]);

        // pod/name, deployment/name, service/name
        command.arg(target);

        // Apply the port bindings
        for port in &config.ports {
            let value = if let Some(local) = port.local {
                format!("{local}:{remote}", remote = port.remote)
            } else {
                format!(":{remote}", remote = port.remote)
            };

            command.arg(&value);
        }

        // TODO: Handle invalid addresses (e.g. not an IP, not "localhost", ...)
        // TODO: Handle invalid port configurations

        // Create channels for communication
        let (stdout_tx, stdout_rx) = mpsc::channel();
        let (stderr_tx, stderr_rx) = mpsc::channel();
        let (status_tx, status_rx) = mpsc::channel();

        let child_thread = thread::spawn(move || {
            let mut child = command.spawn()?;

            // Read stdout and stderr in separate threads.
            Self::handle_pipe(stdout_tx, child.stdout.take());
            Self::handle_pipe(stderr_tx, child.stderr.take());

            // Wait for the child process to finish
            let status = child.wait().expect("Failed to wait for child process");
            status_tx
                .send(status)
                .expect("Failed to send process status");

            if !status.success() {
                todo!("restart the forwarding")
            }

            println!("{display_name}: Process exited with status: {}", status);

            Ok(())
        });

        Ok(child_thread)
    }

    fn handle_pipe<T: Read + Send + 'static>(stdout_tx: Sender<String>, pipe: Option<T>) {
        if let Some(pipe) = pipe {
            thread::spawn(move || {
                let reader = io::BufReader::new(pipe);
                for line in reader.lines() {
                    if line.is_err() {
                        break;
                    }

                    stdout_tx.send(line.unwrap()).ok();
                }
            });
        }
    }
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
