# ModbusMQTT

A bridge between Modbus devices and MQTT.

It is early days, but the plan is:

* Support custom Modbus transports (Sungrow WiNet-S has been implemented)
* Support _setting_ holding registers over MQTT
* Support optional auto-configuration of Home Assistant entities, including using [MQTT Number](https://www.home-assistant.io/integrations/number.mqtt/) et al for holding registers, to allow setting the value.



## Similar projects

* https://github.com/Instathings/modbus2mqtt
* https://github.com/TenySmart/ModbusTCP2MQTT - Sungrow inverter specific
* https://github.com/bohdan-s/SunGather - Sungrow inverter specific

## Example connect config

```json
{
  "host": "10.10.10.219",
  "unit": 1,
  "proto": "tcp",
  "address_offset": -1,
  "input": [{
    "address": 5017,
    "type": "u32",
    "name": "dc_power",
    "swap_words": false,
    "period": "3s"
  },
  {
    "address": 5008,
    "type": "s16",
    "name": "internal_temperature",
    "period": "1m"
  },
  {
    "address": 13008,
    "type": "s32",
    "name": "load_power",
    "swap_words": false,
    "period": "3s"
  },
  {
    "address": 13010,
    "type": "s32",
    "name": "export_power",
    "swap_words": false,
    "period": "3s"
  },
  {
    "address": 13022,
    "name": "battery_power",
    "period": "3s"
  },
  {
    "address": 13023,
    "name": "battery_level",
    "period": "1m"
  },
  {
    "address": 13024,
    "name": "battery_health",
    "period": "10m"
  }],
  "hold":  [{
    "address": 13058,
    "name": "max_soc",
    "period": "90s"
  },
  {
    "address": 13059,
    "name": "min_soc",
    "period": "90s"
  }]
}
```
