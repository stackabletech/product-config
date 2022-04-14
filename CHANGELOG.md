# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [0.4.0] - 2022-04-14

### Added

- Module added for creating Python configuration files for applications based on the Flask App
  Builder ([#60]).

[#60]: https://github.com/stackabletech/product-config/pull/60

## [0.3.1] - 2022-03-15

- replaced `regex` with `fancy-regex` to support look-ahead in unit regexes ([#56])

[#56]: https://github.com/stackabletech/product-config/pull/56

## [0.3.0] - 2021-12-09

### Changed
- `ProductConfigManager::from_yaml_file` now takes a `&Path` as well as `&str` ([#43])

[#43]: https://github.com/stackabletech/product-config/pull/43

## [0.2.0] - 2021-11-05


### Added
- Support to generate Hadoop configuration XML ([#35])
 
[#35]: https://github.com/stackabletech/product-config/pull/35

## [0.1.0] - 2021-09-01

### Added

- Initial release
