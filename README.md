# ModbusMQTT

This repository is a workspace for developing ModbusMQTT and the crates developed in the course of the project.

View the appropriate READMEs in each package directory for details about each.

## [`modbus-mqtt`](./modbus-mqtt)

[![Docker](https://img.shields.io/docker/v/bjeanes/modbus-mqtt?label=docker)](https://hub.docker.com/r/bjeanes/modbus-mqtt)
[![Crates.io](https://img.shields.io/crates/v/modbus-mqtt.svg)](https://crates.io/crates/modbus-mqtt)
[![docs.rs](https://img.shields.io/docsrs/modbus-mqtt)](https://docs.rs/modbus-mqtt/latest/modbus_mqtt/)
![license](https://img.shields.io/crates/l/modbus-mqtt)

ModbusMQTT is a bridge between Modbus devices and MQTT. It aims to allow the operator to generically expose any compatible Modbus device as though its API were MQTT.

## [`sungrow-winets`](./sungrow-winets)

![Crates.io](https://img.shields.io/crates/v/sungrow-winets.svg)
![docs.rs](https://img.shields.io/docsrs/sungrow-winets.svg)
![Crates.io](https://img.shields.io/crates/l/sungrow-winets)

This is a barebones API client for reading and writing settings for Sungrow solar and hybrid inverters equipped with a WiNet-S communications module. Its only known use is `tokio_modbus-winets` (see below).

## [`tokio_modbus-winets`](./tokio_modbus-winets)

![Crates.io](https://img.shields.io/crates/v/tokio_modbus-winets.svg)
![docs.rs](https://img.shields.io/docsrs/tokio_modbus-winets)
![Crates.io](https://img.shields.io/crates/l/tokio_modbus-winets)

This wraps `sungrow-winets` client in the appropriate traits from [`tokio-modbus`](https://crates.io/crates/tokio-modbus) to allow accessing the Modbus registers which the Sungrow WiNet-S dongle exposes. The reason for this is simply that Sungrow's TCP Modbus support is buggy and inconsistent (though improving).
