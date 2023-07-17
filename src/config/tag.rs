use serde::{Deserialize, Deserializer};
use std::ops::Deref;
use std::str::FromStr;

/// A tag
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Tag(#[serde(deserialize_with = "deserialize_tag")] String);

impl Tag {
    pub fn new_unchecked<V: Into<String>>(value: V) -> Self {
        Self(value.into())
    }
}

impl Deref for Tag {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl FromStr for Tag {
    type Err = FromStringError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Ok(Tag::new_unchecked(String::new()));
        }

        let mut chars = value.chars();
        let first = chars.next().expect("tag is not empty");
        if !first.is_ascii_alphabetic() {
            return Err(FromStringError::MustStartAlphabetic(first));
        }

        while let Some(c) = chars.next() {
            if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
                return Err(FromStringError::InvalidCharacter(c));
            }
        }

        Ok(Self::new_unchecked(value))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FromStringError {
    #[error("Tag name must begin with an alphabetic character, got '{0}'")]
    MustStartAlphabetic(char),
    #[error("Tag name must contain only alphanumeric characters, '-' or '_', got '{0}'")]
    InvalidCharacter(char),
}

fn deserialize_tag<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let tag = String::deserialize(deserializer)?;
    if tag.is_empty() {
        return Ok(String::new());
    }

    let mut chars = tag.chars();
    let first = chars.next().expect("tag is not empty");
    if !first.is_ascii_alphabetic() {
        return Err(serde::de::Error::custom(format!(
            "Tag name must begin with an alphabetic character"
        )));
    }

    while let Some(c) = chars.next() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            return Err(serde::de::Error::custom(format!(
                "Tag name must contain only alphanumeric characters, \"-\" or \"_\""
            )));
        }
    }

    Ok(tag)
}
