# ModbusMQTT

![Crates.io](https://img.shields.io/crates/v/modbus-mqtt.svg)
![docs.rs](https://img.shields.io/docsrs/modbus-mqtt)
![Crates.io](https://img.shields.io/crates/l/modbus-mqtt)

A bridge between Modbus devices and MQTT.

It is early days, but the plan is:

* [x] Support custom Modbus transports (Sungrow WiNet-S has been implemented)
  * Modbus RTU has not been tested because I don't have a serial Modbus device, but in principle it should work. Please let me know
* [x] Support reading input registers
* [x] Support reading holding registers
* [ ] Support _setting_ holding registers
* [ ] Support optional auto-configuration of Home Assistant entities, including using [MQTT Number](https://www.home-assistant.io/integrations/number.mqtt/) et al for holding registers, to allow setting the value.
* [ ] TLS MQTT connections
* [ ] WebSocket MQTT connections

NOTE: For the time being, this does not support MQTTv5.

## Installing

For now, use `cargo install` (Rust toolchain required). Soon, I will have release binaries attached to GitHub releases. In the future, there will also be Docker images made available for convenience.

## Running

Start the binary, passing in the URL to your MQTT server, including any credentials:

```sh-session
$ modbus-mqtt mqtt://$MQTT_HOST[:$MQTT_PORT]/[$CUSTOM_MODBUS_TOPIC]
```

The supported protocols are currently just `tcp://`/`mqtt://`, but with intent to support: `mqtts://`, `ssl://`/`tls://`, `ws://`, and `wss://`.

The default topic which ModbusMQTT monitors and to which it publishes is `modbus-mqtt`. You can vary that by changing the path portion of the MQTT URL.

Further, you can change other MQTT options by using query params, such as setting a custom client_id:

```sh
"mqtt://1.2.3.4/?client_id=$CUSTOM_CLIENT_ID"
```

For a full list of supported options, check [the MQTT client library's source code](https://github.com/bytebeamio/rumqtt/blob/c6dc1f7cfb26f6c1f676954a51b398708d49091a/rumqttc/src/lib.rs#L680-L768).

### Connecting to Modbus devices

To connect to a Modbus device, you need to post the connection details to MQTT under a topic of `$prefix/$connection_id/connect`. It is intended that such messages are marked as **retained** so that ModbusMQTT reconnects to your devices when it restarts.

For instance, a simple config might be:

```jsonc
// PUBLISH modbus-mqtt/solar-inverter/connect
{
  "host": "10.10.10.219",
  "proto": "tcp",
}
```

If the connection is successful, you will see the following message like the following sent to the MQTT server:

```jsonc
// modbus-mqtt/solar-inverter/state
"connected"
```

#### Full connection examples

All fields accepted (optional fields show defaults)

```jsonc
{
  // Common fields
  "address_offset": 0, // optional
  "unit": 1,           // optional, aliased to "slave"

  // TCP:
  "proto": "tcp",
  "host": "1.2.3.4",
  "port": 502, // optional

  // RTU / Serial:
  "proto": "rtu",
  "tty": "/dev/ttyACM0",
  "data_bits": "Eight",   // optional (TODO: accept numeric and lowercase)
                          //   valid: Five, Six, Seven, Eight
  "stop_bits": "One",     // optional (TODO: accept numeric and lowercase)
                          //   valid: One, Two
  "flow_control": "None", // optional (TODO: accept lowercase)
                          //   valid: None, Software, Hardware
  "parity": "None",       // optional (TODO: accept lowercase)
                          //   valid: None, Odd, Even

  // Sungrow WiNet-S dongle
  "proto": "winet-s",
  "host": "1.2.3.4",
}
```

#### Monitoring registers

Post to `$MODBUS_MQTT_TOPIC/$CONNECTION_ID/$TYPE/$ADDRESS` where `$TYPE` is one of `input` or `holding` with the following payload (optional fields show defaults):

```jsonc
{
  "name": null,        // OPTIONAL - gives the register a name which is used in the register MQTT topics (must be a valid topic component)

  "interval": "1m",    // OPTIONAL - how often to update the registers value to MQTT
                       //   e.g.: 3s (every 3 seconds)
                       //         2m (every 2 minutes)
                       //         1h (every 1 hour)

  "swap_bytes": false, // OPTIONAL
  "swap_words": false, // OPTIONAL

  "type": "s16",       // OPTIONAL
                       //   valid: s8, s16, s32, s64 (signed)
                       //          u8, u16, u32, u64 (unsigned)
                       //          f32, f64          (floating point)

  "scale": 0,          // OPTIONAL - number in register will be multiplied by 10^(scale)
                       //   e.g.: to turn kW into W, you would provide scale=3
                       //         to turn W into kW, you would provide scale=-3

  "offset": 0,         // OPTIONAL - will be added to the final result (AFTER scaling)


  // Additionally, "type" can be set to "array":
  "type": "array",
  "of": "u16"          // The default array element is u16, but you can change it with the `of` field
}
```

Further, the `type` field can additionally be set to `"array"`, in which case, a `count` field must be provided. The array elements default to `"s16"` but can be overriden in the `"of"` field.

NOTE: this is likely to change such that there is always a `count` field (with default of 1) and if provided to be greater than 1, it will be interpreted to be an array of elements of the `type` specified.

There is some code to accept `"string"` type (with a required `length` field) but this is experimental and untested.

##### Register shorthand

When issuing the `connect` payload, you can optionally include `input` and/or `holding` fields as arrays containing the above register schema, as long as an `address` field is added. When present, these payloads will be replayed to the MQTT server as if the user had specified each register separately, as above.

This is a recommended way to specify connections, but the registers are broken out separately so that they can be dynamically added to too.

## Development

TODO: set up something like https://hub.docker.com/r/oitc/modbus-server to test with

## Similar projects

* https://github.com/Instathings/modbus2mqtt
* https://github.com/TenySmart/ModbusTCP2MQTT - Sungrow inverter specific
* https://github.com/bohdan-s/SunGather - Sungrow inverter specific
