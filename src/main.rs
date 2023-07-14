use crate::kubectl::Kubectl;
use crate::portfwd::PortForwardConfigs;
use anyhow::Result;
use semver::Version;
use std::fs::File;
use std::io::Read;
use std::process::ExitCode;

mod kubectl;
mod portfwd;

static LOGO: &'static str = indoc::indoc!(
    r#"
    ██╗░░██╗░█████╗░░██████╗░░░░░███████╗██╗░░░░░░░██╗██████╗
    ██║░██╔╝██╔══██╗██╔════╝░██╗░██╔════╝██║░░██╗░░██║██╔══██╗
    █████═╝░╚█████╔╝╚█████╗░░╚═╝░█████╗░░╚██╗████╗██╔╝██║░░██║
    ██╔═██╗░██╔══██╗░╚═══██╗░██╗░██╔══╝░░░████╔═████║░██║░░██║
    ██║░╚██╗╚█████╔╝██████╔╝░╚═╝░██║░░░░░░╚██╔╝░╚██╔╝░██████╔╝
    ╚═╝░░╚═╝░╚════╝░╚═════╝░░░░░░╚═╝░░░░░░░╚═╝░░░╚═╝░░╚═════╝"#
);

fn main() -> Result<ExitCode> {
    println!("{}", LOGO.trim_start());
    println!("k8s:fwd {}", env!("CARGO_PKG_VERSION"));

    let lowest_supported_version = Version::new(0, 1, 0);
    let highest_supported_version = lowest_supported_version.clone();

    dotenvy::dotenv().ok();

    // TODO: Allow to specify a configuration as a command-line argument.
    // TODO: Watch the configuration file, stop missing bits and start new ones. (Hash the entries?)

    // TODO: Attempt to find the configuration file in parent directories.
    let mut file = File::open(".k8sfwd")?;
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

    // Ensure kubectl is available.
    let kubectl = Kubectl::new()?;
    match kubectl.version() {
        Ok(version) => println!("Using kubectl version {version}"),
        Err(e) => {
            eprintln!("Failed to locate kubectl: {e}");
            return exitcode(exitcode::UNAVAILABLE);
        }
    }

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

fn exitcode(code: exitcode::ExitCode) -> Result<ExitCode, anyhow::Error> {
    debug_assert!(code <= u8::MAX as i32);
    Ok(ExitCode::from(code as u8))
}
