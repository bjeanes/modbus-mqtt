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