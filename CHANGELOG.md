# Changelog

All notable changes to this project will be documented in this file.
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- [#4](https://github.com/sunsided/k8sfwd/pull/4):
  Added support for configuration files in the user's home and config directories.
- [#5](https://github.com/sunsided/k8sfwd/pull/5):
  Source files from the path hierarchy and special directories are now merged.

### Changed

- The path to the provided or detected source file(s) is now kept relative to the
  current working directory only if it is close. If the file is too many layers
  of nesting away, the canonical path is shown instead of a relative one.

## [0.2.0] - 2023-07-17

### Added

- If only the context or the cluster is specified, the other part will be automatically
  looked up from the current configuration. If a single match is found, its value will
  be explicitly specified to `kubectl`. This should help when changing contexts while
  having a port-forwarding session open as intermittent errors will consistently
  produce the same forwarding rule regardless of the currently active context.
- Added support for tags via the `--tags <tag1> <tag2> ...` command-line option. Only
  targets matching any one of the specified tags will be forwarded.

### Fixed

- Only default to current cluster when neither context nor cluster is specified.
  Previously, specifying only one of the values would result in the other
  value being automatically filled from the currently active context, resulting in
  possibly invalid combinations.

## [0.1.0] - 2023-07-16

### Internal

- 🎉 Initial release with support for hierarchical `.k8sfwd` detection.

[0.2.0]: https://github.com/sunsided/k8sfwd/releases/tag/0.2.0
[0.1.0]: https://github.com/sunsided/k8sfwd/releases/tag/0.1.0
