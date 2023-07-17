use crate::config::Tag;
use serde::{de, Deserialize, Deserializer};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::ops::Deref;
use std::str::FromStr;

/// A tag selection, e.g. `foo` or `foo+bar+baz`.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct TagSelection(HashSet<Tag>);

impl TagSelection {
    pub fn matches(&self, values: &HashSet<Tag>) -> bool {
        self.0.is_subset(values)
    }
}

pub trait TagSelectionUtils {
    fn matches_tags(&self, values: &HashSet<Tag>) -> bool;
}

impl TagSelectionUtils for Vec<TagSelection> {
    fn matches_tags(&self, values: &HashSet<Tag>) -> bool {
        self.iter().any(|s| s.matches(&values))
    }
}

impl Deref for TagSelection {
    type Target = HashSet<Tag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for TagSelection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut vec = Vec::from_iter(self.0.iter());
        vec.sort();
        for tag in vec {
            tag.hash(state);
        }
    }
}

impl FromIterator<Tag> for TagSelection {
    fn from_iter<T: IntoIterator<Item = Tag>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl FromStr for TagSelection {
    type Err = FromStringError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Ok(TagSelection::default());
        }

        let parts = value.split('+');
        let names: HashSet<String> = parts
            .filter(|&c| !c.contains('+'))
            .filter(|&c| !c.is_empty())
            .map(|c| c.into())
            .collect();

        if names.is_empty() {
            return Ok(TagSelection::default());
        }

        let mut tags = HashSet::new();
        for name in names.into_iter() {
            tags.insert(Tag::from_str(&name)?);
        }

        Ok(Self(tags))
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum FromStringError {
    #[error("Invalid tag: {0}")]
    InvalidTag(#[from] crate::config::tag::FromStringError),
}

impl<'de> Deserialize<'de> for TagSelection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let input = String::deserialize(deserializer)?;
        match TagSelection::from_str(&input) {
            Ok(tags) => Ok(tags),
            Err(e) => Err(de::Error::custom(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let tags: TagSelection = TagSelection::from_str("").unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn test_trivial() {
        let tags: TagSelection = serde_yaml::from_str("foo").unwrap();
        assert!(tags.contains(&Tag::new_unchecked("foo")));
    }

    #[test]
    fn test_complex() {
        let tags: TagSelection = serde_yaml::from_str("foo+bar+++baz++").unwrap();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&Tag::new_unchecked("foo")));
        assert!(tags.contains(&Tag::new_unchecked("bar")));
        assert!(tags.contains(&Tag::new_unchecked("baz")));
    }

    #[test]
    fn test_invalid() {
        let tags = TagSelection::from_str("foo+#baz");
        assert_eq!(
            tags,
            Err(FromStringError::InvalidTag(
                crate::config::tag::FromStringError::MustStartAlphabetic('#')
            ))
        );
    }

    #[test]
    fn test_matches() {
        let selections = vec![
            TagSelection::from_str("foo+bar").unwrap(),
            TagSelection::from_str("baz").unwrap(),
        ];

        // foo+bar are present, so is baz
        assert!(selections.matches_tags(&HashSet::from_iter([
            Tag::new_unchecked("foo"),
            Tag::new_unchecked("bar"),
            Tag::new_unchecked("baz"),
        ])));

        // baz is present
        assert!(selections.matches_tags(&HashSet::from_iter([Tag::new_unchecked("baz"),])));

        // foo+bar are present
        assert!(selections.matches_tags(&HashSet::from_iter([
            Tag::new_unchecked("foo"),
            Tag::new_unchecked("bar"),
        ])));

        // baz present
        assert!(selections.matches_tags(&HashSet::from_iter([
            Tag::new_unchecked("foo"),
            Tag::new_unchecked("baz"),
        ])));

        // neither foo+bar, nor baz are present.
        assert!(!selections.matches_tags(&HashSet::from_iter([
            Tag::new_unchecked("foo"),
            Tag::new_unchecked("bang"),
        ])));
    }
}
