use lazy_static::lazy_static;
use semver::Version;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::fs::File;
use std::io;
use std::io::Read;
use std::net::IpAddr;

lazy_static! {
    pub static ref LOWEST_SUPPORTED_VERSION: Version = Version::new(0, 1, 0);
    pub static ref HIGHEST_SUPPORTED_VERSION: Version = Version::new(0, 1, 0);
}

#[derive(Debug, Deserialize)]
pub struct PortForwardConfigs {
    pub version: Version,
    pub targets: Vec<PortForwardConfig>,
}

#[derive(Debug, Deserialize)]
pub struct PortForwardConfig {
    /// An optional name used to refer to this configuration.
    pub name: Option<String>,
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

/// The type of resource to forward to.
#[derive(Debug, Deserialize)]
pub enum ResourceType {
    #[serde(rename = "service")]
    Service,
    #[serde(rename = "deployment")]
    Deployment,
    #[serde(rename = "pod")]
    Pod,
}

/// A port to forward.
#[derive(Debug)]
pub struct Port {
    /// The local port to forward to.
    pub local: Option<u16>,
    /// The remote port to forward to.
    pub remote: u16,
}

impl Default for ResourceType {
    fn default() -> Self {
        Self::Service
    }
}

fn default_namespace() -> String {
    "default".to_string()
}

impl ResourceType {
    pub fn to_arg(&self) -> &'static str {
        match self {
            ResourceType::Service => "service",
            ResourceType::Deployment => "deployment",
            ResourceType::Pod => "pod",
        }
    }
}

impl IntoIterator for PortForwardConfigs {
    type Item = PortForwardConfig;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.targets.into_iter()
    }
}

impl PortForwardConfigs {
    pub fn is_supported_version(&self) -> bool {
        #[allow(clippy::absurd_extreme_comparisons)]
        if self.version < *LOWEST_SUPPORTED_VERSION || self.version > *HIGHEST_SUPPORTED_VERSION {
            false
        } else {
            true
        }
    }
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

impl<'de> Deserialize<'de> for Port {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PortVisitor;

        impl<'de> serde::de::Visitor<'de> for PortVisitor {
            type Value = Port;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or an object")
            }

            fn visit_i16<E>(self, remote: i16) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if remote <= 0 {
                    return Err(E::custom("Invalid port number: value must be positive"));
                }

                Ok(Port {
                    local: None,
                    remote: remote as _,
                })
            }

            fn visit_u16<E>(self, remote: u16) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if remote == 0 {
                    return Err(E::custom("Invalid port number: value must be positive"));
                }

                Ok(Port {
                    local: None,
                    remote,
                })
            }

            fn visit_u64<E>(self, remote: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if remote == 0 {
                    return Err(E::custom("Invalid port number: value must be positive"));
                }

                if remote > u16::MAX as _ {
                    return Err(E::custom(
                        "Invalid port number: value must be smaller than or equal to 65535",
                    ));
                }

                Ok(Port {
                    local: None,
                    remote: remote as _,
                })
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                // Split the string by ':' and parse the numbers
                let parts: Vec<&str> = s.split(':').collect();
                match parts[..] {
                    [local, remote] => {
                        let local = match local {
                            "" => None,
                            value => Some(value.parse::<u16>().map_err(E::custom)?),
                        };
                        let remote = remote.parse::<u16>().map_err(E::custom)?;

                        Ok(Port { local, remote })
                    }
                    [remote] => {
                        let remote = remote.parse::<u16>().map_err(E::custom)?;
                        Ok(Port {
                            local: None,
                            remote,
                        })
                    }
                    _ => Err(E::custom("Invalid string format")),
                }
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                // Deserialize the JSON object
                let mut local = None;
                let mut remote = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "local" => {
                            if local.is_some() {
                                return Err(Error::duplicate_field("local"));
                            }
                            local = Some(map.next_value()?);
                        }
                        "remote" => {
                            if remote.is_some() {
                                return Err(Error::duplicate_field("remote"));
                            }
                            remote = Some(map.next_value()?);
                        }
                        _ => return Err(Error::unknown_field(&key, &["local", "remote"])),
                    }
                }

                Ok(Port {
                    local,
                    remote: remote.ok_or_else(|| Error::missing_field("remote"))?,
                })
            }
        }

        deserializer.deserialize_any(PortVisitor)
    }
}

pub trait FromYaml {
    fn into_configuration(self) -> Result<PortForwardConfigs, FromYamlError>;
}

impl FromYaml for File {
    fn into_configuration(mut self) -> Result<PortForwardConfigs, FromYamlError> {
        let mut contents = String::new();
        self.read_to_string(&mut contents)?;
        Ok(serde_yaml::from_str(&contents)?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FromYamlError {
    #[error(transparent)]
    InvalidConfiguration(#[from] serde_yaml::Error),
    #[error(transparent)]
    FileReadFailed(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_from_object() {
        let input = r"
            local: 5012
            remote: 80
        ";

        let port: Port = serde_yaml::from_str(input).unwrap();
        assert_eq!(port.local, Some(5012));
        assert_eq!(port.remote, 80);
    }

    #[test]
    fn test_auto_port_from_object() {
        let input = "remote: 80";
        let port: Port = serde_yaml::from_str(input).unwrap();
        assert_eq!(port.local, None);
        assert_eq!(port.remote, 80);
    }

    #[test]
    fn test_port_from_string() {
        let port: Port = serde_yaml::from_str("5012:80").unwrap();
        assert_eq!(port.local, Some(5012));
        assert_eq!(port.remote, 80);
    }

    #[test]
    fn test_auto_port_from_string() {
        let port: Port = serde_yaml::from_str("\":80\"").unwrap();
        assert_eq!(port.local, None);
        assert_eq!(port.remote, 80);
    }

    #[test]
    fn test_auto_port_from_string_2() {
        let port: Port = serde_yaml::from_str("\"80\"").unwrap();
        assert_eq!(port.local, None);
        assert_eq!(port.remote, 80);
    }

    #[test]
    fn test_auto_port_from_string_3() {
        let port: Port = serde_yaml::from_str(":80").unwrap();
        assert_eq!(port.local, None);
        assert_eq!(port.remote, 80);
    }

    #[test]
    fn test_auto_port_from_string_4() {
        let port: Port = serde_yaml::from_str("80").unwrap();
        assert_eq!(port.local, None);
        assert_eq!(port.remote, 80);
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

    #[test]
    fn test_entire_config() {
        let config = r#"
            version: 0.1.0
            targets:
              - name: Test API (Staging)
                target: foo
                type: service
                namespace: bar
                context: null
                cluster: null
                listen_addrs:
                  - "127.0.0.1"
                ports:
                  - "5012:80"
                  - 8080
              - name: Test API (Production)
                target: foo
                type: service
                namespace: bar
                cluster: production
                listen_addrs:
                  - "127.1.0.1"
                ports:
                  - "5012:80"
        "#;

        let config: PortForwardConfigs = serde_yaml::from_str(config).unwrap();
        assert_eq!(config.targets.len(), 2);
    }
}
