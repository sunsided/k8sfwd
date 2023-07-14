use clap::Parser;
use std::fs::File;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Sets a custom config file to load instead of .k8sfwd
    #[arg(value_name = "FILE", value_parser = config_file_exists)]
    pub config: Option<PathBuf>,

    /// Sets a custom path to the kubectl binary
    #[arg(
        long,
        value_name = "FILE",
        env = "KUBECTL_PATH",
        default_value = "kubectl"
    )]
    pub kubectl: PathBuf,
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
