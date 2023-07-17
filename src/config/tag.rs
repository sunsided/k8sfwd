use serde::{de, Deserialize, Deserializer};
use std::ops::Deref;
use std::str::FromStr;

/// A tag
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Tag(String);

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

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum FromStringError {
    #[error("Tag name must begin with an alphabetic character, got '{0}'")]
    MustStartAlphabetic(char),
    #[error("Tag name must contain only alphanumeric characters, '-' or '_', got '{0}'")]
    InvalidCharacter(char),
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tag = String::deserialize(deserializer)?;
        match Tag::from_str(&tag) {
            Ok(tag) => Ok(tag),
            Err(e) => Err(de::Error::custom(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trivial() {
        let tag: Tag = serde_yaml::from_str("foo").unwrap();
        assert_eq!(tag, Tag::new_unchecked("foo"));
    }

    #[test]
    fn test_complex() {
        let tag: Tag = serde_yaml::from_str("fOo_bAR-12-_-").unwrap();
        assert_eq!(tag, Tag::new_unchecked("fOo_bAR-12-_-"));
    }

    #[test]
    fn test_invalid() {
        let tag = serde_yaml::from_str::<Tag>("123");
        assert!(tag.is_err());
    }
}
