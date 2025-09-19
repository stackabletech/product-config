# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## Changed

- Bump `fancy-regex`, `schemars`, `xml` and `rstest` ([#112]).

[#112]: https://github.com/stackabletech/product-config/pull/112

## [0.7.0] - 2024-07-09

### Changed

- Bump Rust dependencies and Action versions ([#98], [#99], [#100], [#101],
  [#102], [#103], [#104], [#105], [#108]).
- Update Renovate config ([#106]).
- Bump Rust version to 1.79.0 ([#110]).

[#98]: https://github.com/stackabletech/product-config/pull/98
[#99]: https://github.com/stackabletech/product-config/pull/99
[#100]: https://github.com/stackabletech/product-config/pull/100
[#101]: https://github.com/stackabletech/product-config/pull/101
[#102]: https://github.com/stackabletech/product-config/pull/102
[#103]: https://github.com/stackabletech/product-config/pull/103
[#104]: https://github.com/stackabletech/product-config/pull/104
[#105]: https://github.com/stackabletech/product-config/pull/105
[#106]: https://github.com/stackabletech/product-config/pull/106
[#108]: https://github.com/stackabletech/product-config/pull/108
[#110]: https://github.com/stackabletech/product-config/pull/110

## [0.6.0] - 2023-10-30

### Changed

- Improved error messages using `snafu` instead of `thiserror` ([#96]).

[#96]: https://github.com/stackabletech/product-config/pull/96

## [0.5.0] - 2022-08-16

### Changed

- Quotes are no longer emitted for empty strings in java.properties files ([#88]).

[#88]: https://github.com/stackabletech/product-config/pull/88

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
