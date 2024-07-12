# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog],
and this project adheres to [Semantic Versioning].

## [Unreleased]

### Fixed

- It is able to connect on WiNet-S devices with updated firmware, which require logging in with username and password

### Breaking

- Because the client now needs to take username and password to work with new devices, client creation has to change.
  Now this has a `ClientBuilder` to optionally provide username and password (it defaults to the default credentials).
- `Client` needs to be `mut` (for now -- hope to change this again)

## [0.2.0] - 2023-05-22

### Changed

- Depedency upgrades
- Refactoring

## [0.1.0] - 2022-08-30

- Initial release

<!-- Links -->

[keep a changelog]: https://keepachangelog.com/en/1.0.0/
[semantic versioning]: https://semver.org/spec/v2.0.0.html

<!-- Versions -->

[Unreleased]: https://github.com/bjeanes/modbus-mqtt/compare/sungrow-winets-v0.2.0...HEAD
[0.2.0]: https://github.com/bjeanes/modbus-mqtt/releases/tag/sungrow-winets-v0.2.0
[0.1.0]: https://github.com/bjeanes/modbus-mqtt/releases/tag/sungrow-winets-v0.1.0
