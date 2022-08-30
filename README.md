# ModbusMQTT

Very rough for now.

## Topic spec

```
prefix/status -> { "status": "running" } # last-will message here too

prefix/connect/<connection> <- {
        "host": "localhost",
        "port": 502,
        "slave": 1,
    }

prefix/status/<connection> -> {
        "host": "localhost",
        "port": 502,
        "slave": 1,
        "status": "connected"
    }

prefix/logs/<connection> -> { "message": "log message", level: "level" }

prefix/connection/<connection>/monitor[/opt-name] <- {
        "address": 5100,
        "type": "holding|input",
        "count": 1,
        "interval": 10, // seconds
}
```

## Similar projects

* https://github.com/Instathings/modbus2mqtt
* https://github.com/TenySmart/ModbusTCP2MQTT - Sungrow inverter specific

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