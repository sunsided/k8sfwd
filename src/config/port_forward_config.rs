// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use crate::config::{MergeWith, Port, ResourceType};
use just_a_tag::Tag;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct PortForwardConfig {
    /// Designates the file from which this configuration was loaded.
    #[serde(skip_serializing, skip_deserializing)]
    pub source_file: Option<PathBuf>,
    /// An optional name used to refer to this configuration.
    pub name: Option<String>,
    // TODO: Add alias for filtering
    // TODO: Add explicit/implicit configurations
    /// An optional set of tags to apply to the configuration.
    #[serde(default)]
    pub tags: HashSet<Tag>,
    /// The name of the kubeconfig context to use.
    pub context: Option<String>,
    /// The name of the kubeconfig cluster to use.
    pub cluster: Option<String>,
    /// The addresses or host names to listen on; must be an IP address or `localhost`.
    #[serde(default, deserialize_with = "deserialize_listen_addrs")]
    pub listen_addrs: Vec<String>, // TODO: Make HashSet
    /// The namespace to forward to, e.g. `default`.
    #[serde(default = "default_namespace")]
    pub namespace: String,
    /// The type of resource to forward to.
    #[serde(default)]
    pub r#type: ResourceType,
    /// The name of the resource to forward to.
    pub target: String,
    /// The port to forward.
    pub ports: Vec<Port>, // TODO: Make HashSet
}

impl PartialEq for PortForwardConfig {
    fn eq(&self, other: &Self) -> bool {
        self.target == other.target
    }
}

impl MergeWith for PortForwardConfig {
    fn merge_with(&mut self, other: &Self) {
        self.source_file = other.source_file.clone();
        self.name.merge_with(&other.name);
        self.tags.merge_with(&other.tags);
        self.context.merge_with(&other.context);
        self.cluster.merge_with(&other.cluster);
        self.merge_listen_addrs(&other.listen_addrs);
        self.namespace = other.namespace.clone();
        self.r#type = other.r#type;
        self.target = other.target.clone();
        self.ports.merge_with(&other.ports);
    }
}

impl MergeWith for Vec<PortForwardConfig> {
    fn merge_with(&mut self, other: &Self) {
        if other.is_empty() {
            return;
        }

        // TODO: Ensure sort order is stable.

        let mut map = HashMap::<String, PortForwardConfig>::new();
        for cfg in self.drain(0..) {
            map.insert(cfg.target.clone(), cfg);
        }

        for cfg in other {
            map.entry(cfg.target.clone())
                .and_modify(|current| current.merge_with(cfg))
                .or_insert(cfg.clone());
        }

        *self = Vec::from_iter(map.into_values());
    }
}

impl PortForwardConfig {
    pub fn set_source_file(&mut self, file: PathBuf) {
        self.source_file = Some(file);
    }

    fn merge_listen_addrs(&mut self, other: &[String]) {
        let set: HashSet<String> = HashSet::from_iter(self.listen_addrs.drain(0..));
        let other_set = HashSet::from_iter(other.iter().cloned());
        self.listen_addrs = Vec::from_iter(&mut set.union(&other_set).cloned());
    }
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

        assert_eq!(
            config.tags,
            HashSet::from([Tag::new("foo"), Tag::new("bar")])
        )
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
