# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog],
and this project adheres to [Semantic Versioning].

## [Unreleased]

### Deprecated

- Separate `holding` and `input` sections, in favour of specifying `register_type` field on the register definition to either `"input"` (default) or `"holding"`.

## [0.2.0] - 2022-09-09

### Changed

- README better documents usage
- Massive refactor by @bjeanes in https://github.com/bjeanes/modbus-mqtt/pull/4

### Breaking

- Topic paths have changed
- CLI options have changed (takes a single MQTT URL now)

## [0.1.0] - 2022-08-30

- Initial release
- Basic support for monitoring a Modbus device and publishing register values to MQTT, including parsing numerics

<!-- Links -->
[keep a changelog]: https://keepachangelog.com/en/1.0.0/
[semantic versioning]: https://semver.org/spec/v2.0.0.html

<!-- Versions -->
[unreleased]: https://github.com/bjeanes/modbus-mqtt/compare/modbus-mqtt-v0.2.0...HEAD
[0.2.0]: https://github.com/bjeanes/modbus-mqtt/compare/modbus-mqtt-v0.1.0...modbus-mqtt-v0.2.0
[0.1.0]: https://github.com/bjeanes/modbus-mqtt/releases/tag/modbus-mqtt-v0.1.0