use crate::config::{Port, ResourceType};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;
use std::net::IpAddr;

#[derive(Debug, Clone, Deserialize)]
pub struct PortForwardConfig {
    /// An optional name used to refer to this configuration.
    pub name: Option<String>,
    /// An optional set of tags to apply to the configuration.
    #[serde(default)]
    pub tags: HashSet<String>, // TODO: Ensure tags use safe characters only
    /// The name of the kubeconfig context to use.
    pub context: Option<String>,
    /// The name of the kubeconfig cluster to use.
    pub cluster: Option<String>,
    /// The addresses or host names to listen on; must be an IP address or `localhost`.
    #[serde(default, deserialize_with = "deserialize_listen_addrs")]
    pub listen_addrs: Vec<String>,
    /// The namespace to forward to, e.g. `default`.
    #[serde(default = "default_namespace")]
    pub namespace: String,
    /// The type of resource to forward to.
    #[serde(default)]
    pub r#type: ResourceType,
    /// The name of the resource to forward to.
    pub target: String,
    /// The port to forward.
    pub ports: Vec<Port>,
}

fn default_namespace() -> String {
    "default".to_string()
}

/// Parses a vector of IP addresses or the literal `localhost`.
fn deserialize_listen_addrs<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper(#[serde(deserialize_with = "deserialize_listen_addr")] String);

    let v = Vec::deserialize(deserializer)?;
    Ok(v.into_iter().map(|Wrapper(a)| a).collect())
}

/// Parses an IPv4 or IPv6 address or the literal `localhost`.
fn deserialize_listen_addr<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;

    if buf == "localhost" {
        return Ok(buf);
    }

    if buf.starts_with('[') && buf.ends_with(']') {
        let ip = &buf[1..(buf.len() - 1)];
        return if ip.parse::<IpAddr>().is_ok() {
            Ok(buf)
        } else {
            Err(Error::custom(format!(
                "An invalid IPv6 address was specified: {buf}"
            )))
        };
    }

    if buf.parse::<IpAddr>().is_ok() {
        return Ok(buf);
    }

    Err(Error::custom(
        "Listen address must be either \"localhost\" or a valid IP address",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tags() {
        let config = serde_yaml::from_str::<PortForwardConfig>(
            r#"
            target: foo
            tags:
              - foo
              - bar
            ports:
              - "1234:5678"
        "#,
        )
        .unwrap();

        assert_eq!(config.tags, HashSet::from(["foo".into(), "bar".into()]))
    }

    #[test]
    fn test_listen_ip_and_localhost() {
        serde_yaml::from_str::<PortForwardConfig>(
            r#"
            target: foo
            listen_addrs:
              - "127.0.0.1"
              - "[::1]"
              - "localhost"
            ports:
              - "1234:5678"
        "#,
        )
        .expect("configuration is valid");
    }

    #[test]
    fn test_listen_invalid_host() {
        serde_yaml::from_str::<PortForwardConfig>(
            r#"
            target: foo
            listen_addrs:
              - "foo"
            ports:
              - "1234:5678"
        "#,
        )
        .expect_err("literal host names must be exactly `localhost`");
    }

    #[test]
    fn test_listen_invalid_ipv4() {
        serde_yaml::from_str::<PortForwardConfig>(
            r#"
            target: foo
            listen_addrs:
              - "127.0.0.256"
            ports:
              - "1234:5678"
        "#,
        )
        .expect_err("the IPv6 address is invalid");
    }

    #[test]
    fn test_listen_invalid_ipv6() {
        serde_yaml::from_str::<PortForwardConfig>(
            r#"
            target: foo
            listen_addrs:
              - "[fe80:2030:31:24]"
            ports:
              - "1234:5678"
        "#,
        )
        .expect_err("the IPv6 address is invalid");
    }
}
