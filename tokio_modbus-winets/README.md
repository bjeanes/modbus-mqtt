## `tokio_modbus-winets`

This wraps `sungrow-winets` client in the appropriate traits from [`tokio-modbus`](https://crates.io/crates/tokio-modbus) to allow accessing the Modbus registers which the Sungrow WiNet-S dongle exposes. The reason for this is simply that Sungrow's TCP Modbus support is buggy and inconsistent (though improving).

![Crates.io](https://img.shields.io/crates/v/tokio_modbus-winets.svg)
![docs.rs](https://img.shields.io/docsrs/tokio_modbus-winets)
![Crates.io](https://img.shields.io/crates/l/tokio_modbus-winets)
