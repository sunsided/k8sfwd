# Changelog

All notable changes to this project will be documented in this file.
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Fixed

- Only default to current cluster when neither context nor cluster is specified.
  Previously, specifying only one of the values would result in the other
  value being automatically filled from the currently active context, resulting in
  possibly invalid combinations.

## [0.1.0] - 2023-07-16

### Internal

- 🎉 Initial release with support for hierarchical `.k8sfwd` detection.

[0.1.0]: https://github.com/sunsided/k8sfwd/releases/tag/0.1.0
