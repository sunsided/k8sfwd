// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use crate::config::MergeWith;
use serde::de::Error;
use serde::{Deserialize, Deserializer};

/// A port to forward.
#[derive(Debug, Copy, Clone)]
pub struct Port {
    /// The local port to forward to.
    pub local: Option<u16>,
    /// The remote port to forward to.
    pub remote: u16,
}

impl MergeWith for Vec<Port> {
    fn merge_with(&mut self, other: &Self) {
        if other.is_empty() {
            return;
        }

        todo!("port merging not implemented")
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
}
