use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct PortForwardConfigs {
    #[serde(default)]
    pub version: u32,
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
    #[serde(default)]
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
                E: serde::de::Error,
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
                E: serde::de::Error,
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
                E: serde::de::Error,
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
                E: serde::de::Error,
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
                                return Err(serde::de::Error::duplicate_field("local"));
                            }
                            local = Some(map.next_value()?);
                        }
                        "remote" => {
                            if remote.is_some() {
                                return Err(serde::de::Error::duplicate_field("remote"));
                            }
                            remote = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(serde::de::Error::unknown_field(&key, &["local", "remote"]))
                        }
                    }
                }

                Ok(Port {
                    local,
                    remote: remote.ok_or_else(|| serde::de::Error::missing_field("remote"))?,
                })
            }
        }

        deserializer.deserialize_any(PortVisitor)
    }
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
    fn test_entire_config() {
        let config = r#"
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
