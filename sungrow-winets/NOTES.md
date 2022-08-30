# Requests

Of note:

* The responses are pretty similar between websocket requests and HTTP requests
* `result_code`:
	* 1 - success
	* 106 - invalid or expired token
	* 200 - ?
	* 301 - message: "I18N_COMMON_READ_FAILED" - seem to be fleeting and also corresponds with HTTP request error `requestError { ..., source: hyper::Error(IncompleteMessage) }` which seems to imply the connection was closed abruptly by the server.
	* 391 - I18N_COMMON_SET_FAILED

	* varying other values for different types of errors
* The `result_msg` is a usually "success", but contains more detail for certain errors. In at least one observed error
  response, the key was missing entirely.

## HTTP

The requests in the Web UI often have other parameters, including the token included. But if they are omitted below,
it's because they were not found to be necessary.

### Key translations

```sh-session
❯ curl http://$INVERTER_IP/i18n/en_US.properties
I18N_COMMON_SENIOR_SET_TEN_ENABLE=10 Min Over Vtg En.
I18N_COMMON_AB_VOLTAGE=A-B Line Voltage
I18N_CONFIG_KEY_796=AFCI Self Inspection Failure
I18N_COMMON_A_PHARE_POWER=Phase A Active Power
I18N_COMMON_BC_VOLTAGE=B-C Line Voltage
I18N_CONFIG_KEY_854=Bin Document CRC Checkout Error
I18N_COMMON_B_PHARE_POWER=Phase B Active Power
I18N_COMMON_CA_VOLTAGE=C-A Line Voltage
I18N_COMMON_C_PHARE_POWER=Phase C Active Power
...
```

### About

```sh-session
❯ curl http://$INVERTER_IP/about/list
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"list":	[{
				"data_name":	"I18N_COMMON_DEVICE_SN",
				"data_value":	"REDACTED",
				"data_unit":	"",
				"type":	"1"
			}, {
				"data_name":	"I18N_COMMON_APPLI_SOFT_VERSION",
				"data_value":	"WINET-SV200.001.00.P012",
				"data_unit":	"",
				"type":	"2"
			}, {
				"data_name":	"I18N_COMMON_BUILD_SOFT_VERSION",
				"data_value":	"WINET-SV200.001.00.B001",
				"data_unit":	"",
				"type":	"2"
			}, {
				"data_name":	"I18N_COMMON_VERSION",
				"data_value":	"M_WiNet-S_V01_V01_A",
				"data_unit":	"",
				"type":	"0"
			}]
	}
}
```

### Device Types

This seems to return the values used in `dev_id` field for devices from list

```sh-session
❯ curl http://$INVERTER_IP/device/getType
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"count":	5,
		"list":	[{
				"name":	"I18N_COMMON_STRING_INVERTER",
				"value":	1
			}, {
				"name":	"I18N_COMMON_SOLAR_INVERTER",
				"value":	21
			}, {
				"name":	"I18N_COMMON_STORE_INVERTER",
				"value":	35
			}, {
				"name":	"I18N_COMMON_AMMETER",
				"value":	18
			}, {
				"name":	"I18N_COMMON_CHARGING_PILE",
				"value":	46
			}]
	}
}
```

### Product List

### Device List

```sh-session
❯ curl http://$INVERTER_IP/inverter/list -X POST
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"list":	[{
				"id":	1,
				"dev_id":	1,
				"dev_code":	3343,
				"dev_type":	35,
				"dev_procotol":	2,
				"inv_type":	0,
				"dev_sn":	"REDACTED",
				"dev_name":	"SH5.0RS(COM1-001)",
				"dev_model":	"SH5.0RS",
				"port_name":	"COM1",
				"phys_addr":	"1",
				"logc_addr":	"1",
				"link_status":	1,
				"init_status":	1,
				"dev_special":	"0"
			}, {
				"id":	2,
				"dev_id":	2,
				"dev_code":	8424,
				"dev_type":	44,
				"dev_procotol":	0,
				"inv_type":	0,
				"dev_sn":	"REDACTED",
				"dev_name":	"SBR128(COM1-200)",
				"dev_model":	"SBR128",
				"port_name":	"COM1",
				"phys_addr":	"200",
				"logc_addr":	"2",
				"link_status":	1,
				"init_status":	255,
				"dev_special":	"0"
			}],
		"count":	2
	}
}
```

See also device listing over websocket.

### Time

Weirdly, this needs to be authenticated of all things.

```sh-session
❯ curl http://$INVERTER_IP/time/get?token=$TOKEN
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"time":	"2022-08-29 20:14",
		"sync_device":	"0",
		"dispatching_mode":	"0",
		"ntp_server_jp":	"re-ene.kyuden.co.jp",
		"curr_timezone":	"UTC+10:00",
		"source":	"7",
		"ntp_server":	"au.pool.ntp.org",
		"ntp_port":	"123",
		"ntp_interval":	"5",
		"ntp_timestamp":	"2022-08-29 20:12:44",
		"tz_reboot_flag":	"0",
		"data_name":	"I18N_COMMON_LONGITUDE",
		"data_value":	"--",
		"data_unit":	"",
		"data_name":	"I18N_COMMON_LATITUDE",
		"data_value":	"--",
		"data_unit":	"",
		"timezone_gps":	"UTC"
	}
}
```

### Overview

```sh-session
❯ curl http://$INVERTER_IP/device/overview?token=fd919fa6-6ff4-46ac-90c5-6d367edc84ad
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"module_info":	{
			"module_sn":	"REDACTED",
			"module_ver":	"M_WiNet-S_V01_V01_A"
		},
		"net_info":	{
			"wifi_conn_sts":	0,
			"eth_conn_sts":	1,
			"eth2_conn_sts":	0,
			"wifi_cmd":	170
		},
		"remote_info":	{
			"module_sn":	"REDACTED",
			"ip":	"app.isolarcloud.com"
		},
		"sys_time":	{
			"sync_device":	0,
			"time":	"2022-08-29 20:19",
			"timezone":	"UTC+10:00"
		},
		"list":	[{
				"dev_name":	"SH5.0RS(COM1-001)",
				"dev_sn":	"REDACTED",
				"link_status":	1,
				"country_code":	6,
				"country":	"I18N_COMMON_AUSTRALIA",
				"company":	"AS/NZS 4777.2:2020 Australia A",
				"company_code":	"13"
			}]
	}
}
```

### Get Initial Parameters

Used to generate a Word Doc report, based on the template at `/template.docx`.

```sh-session
❯ curl http://$INVERTER_IP/device/getInitParam?token=$TOKEN
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"list":	[{
				"dev_name":	"SH5.0RS(COM1-001)",
				"dev_sn":	"REDACTED",
				"list":	[{
						"param_addr":	31605,
						"param_name":	"I18N_COMMON_REACTIVE_REGULATION_MODE",
						"param_value":	"164",
						"unit":	"",
						"value_name":	"Q(U)"
					}, {
						"param_addr":	31700,
						"param_name":	"I18N_COMMON_Q_U_CURVE",
						"param_value":	"0",
						"unit":	"",
						"value_name":	"I18N_COMMON_A_CURVE"
					}, {
						"param_addr":	31712,
						"param_name":	"QU_EnableMode",
						"param_value":	"170",
						"unit":	"",
						"value_name":	"I18N_COMMON_YES"
					}, {
						"param_addr":	32578,
						"param_name":	"I18N_10RT_RNNN_1527766",
						"param_value":	"162",
						"unit":	"",
						"value_name":	"I18N_COMMON_MAXIMUM_POWER"
					}, {
						"param_addr":	30092,
						"param_name":	"I18N_COMMON_FAULT_RECOVERY_TIME",
						"param_value":	"60",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	31400,
						"param_name":	"I18N_COMMON_FREQUENCY_DROP_STATUS",
						"param_value":	"170",
						"unit":	"",
						"value_name":	"I18N_COMMON_ENABLE"
					}, {
						"param_addr":	31404,
						"param_name":	"F1",
						"param_value":	"50.25",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	31405,
						"param_name":	"F2",
						"param_value":	"50.75",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	31406,
						"param_name":	"F3",
						"param_value":	"52.00",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	31409,
						"param_name":	"P1",
						"param_value":	"200.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31410,
						"param_name":	"P2",
						"param_value":	"100.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31411,
						"param_name":	"P3",
						"param_value":	"0.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31412,
						"param_name":	"I18N_COMMON_OVER_FREQUENCY_DROP_RECOVERY_POINT",
						"param_value":	"50.15",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	31413,
						"param_name":	"I18N_COMMON_OVER_FREQUENCY_DROP_CURVE",
						"param_value":	"1",
						"unit":	"",
						"value_name":	"I18N_COMMON_B_CURVE"
					}, {
						"param_addr":	31414,
						"param_name":	"I18N_COMMON_OVER_FREQUENCY_DROP_ACTIVE_RATE",
						"param_value":	"6000",
						"unit":	"%/min",
						"value_name":	""
					}, {
						"param_addr":	31415,
						"param_name":	"I18N_COMMON_OVER_FREQUENCY_DROP_WAIT_RESTORE_TIME",
						"param_value":	"20.0",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	31416,
						"param_name":	"I18N_COMMON_OVER_FREQUENCY_DROP_ACTIVE_RESTORE_RATE",
						"param_value":	"16",
						"unit":	"%/min",
						"value_name":	""
					}, {
						"param_addr":	31417,
						"param_name":	"I18N_COMMON_OVER_FREQUENCY_DROP_RESPONSE_TIME",
						"param_value":	"0.00",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	31420,
						"param_name":	"I18N_COMMON_FRE_INCREMENT",
						"param_value":	"170",
						"unit":	"",
						"value_name":	"I18N_COMMON_ENABLE"
					}, {
						"param_addr":	31421,
						"param_name":	"F1",
						"param_value":	"49.75",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	31422,
						"param_name":	"F2",
						"param_value":	"49.00",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	31423,
						"param_name":	"F3",
						"param_value":	"48.00",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	31427,
						"param_name":	"P1",
						"param_value":	"0.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31428,
						"param_name":	"P2",
						"param_value":	"100.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31429,
						"param_name":	"P3",
						"param_value":	"200.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31433,
						"param_name":	"I18N_COMMON_UNDER_FREQUENCY_UP_RESTORE_POINT",
						"param_value":	"49.85",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	31434,
						"param_name":	"I18N_COMMON_UNDER_FREQUENCY_UP_CURVE",
						"param_value":	"1",
						"unit":	"",
						"value_name":	"I18N_COMMON_B_CURVE"
					}, {
						"param_addr":	31435,
						"param_name":	"I18N_COMMON_UNDER_FREQUENCY_UP_ACTIVE_RATE",
						"param_value":	"6000",
						"unit":	"%/min",
						"value_name":	""
					}, {
						"param_addr":	31436,
						"param_name":	"I18N_COMMON_UNDER_FREQUENCY_UP_WAIT_RESTORE_TIME",
						"param_value":	"20.0",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	31437,
						"param_name":	"I18N_COMMON_UNDER_FREQUENCY_UP_ACTIVE_RESTORE_RATE",
						"param_value":	"16",
						"unit":	"%/min",
						"value_name":	""
					}, {
						"param_addr":	31438,
						"param_name":	"I18N_COMMON_UNDER_FREQUENCY_UP_RESPONSE_TIME",
						"param_value":	"0.00",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	31196,
						"param_name":	"I18N_COMMON_FAULT_ACTIVE_SLOWDOWN",
						"param_value":	"170",
						"unit":	"",
						"value_name":	"I18N_COMMON_ENABLE"
					}, {
						"param_addr":	31197,
						"param_name":	"I18N_COMMON_FAULT_ACTIVE_SLOWDOWN_TIME",
						"param_value":	"360",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	31200,
						"param_name":	"I18N_COMMON_ACTIVE_SPEED_CONTROL",
						"param_value":	"170",
						"unit":	"",
						"value_name":	"I18N_COMMON_ENABLE"
					}, {
						"param_addr":	31201,
						"param_name":	"I18N_COMMON_ACTIVE_REACTIVE_DOWN",
						"param_value":	"16",
						"unit":	"%/min",
						"value_name":	""
					}, {
						"param_addr":	31202,
						"param_name":	"I18N_COMMON_ACTIVE_REACTIVE_UP",
						"param_value":	"16",
						"unit":	"%/min",
						"value_name":	""
					}, {
						"param_addr":	31230,
						"param_name":	"I18N_COMMON_GRID_VOLTAGE_ACTIVE_ADJUST",
						"param_value":	"170",
						"unit":	"",
						"value_name":	"I18N_COMMON_ENABLE"
					}, {
						"param_addr":	31231,
						"param_name":	"OPU_V1",
						"param_value":	"253.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	31232,
						"param_name":	"OPU_V2",
						"param_value":	"260.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	31233,
						"param_name":	"OPU_V3",
						"param_value":	"260.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	31234,
						"param_name":	"OPU_V4",
						"param_value":	"260.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	31235,
						"param_name":	"OPU_P1",
						"param_value":	"100.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31236,
						"param_name":	"OPU_P2",
						"param_value":	"20.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31237,
						"param_name":	"OPU_P3",
						"param_value":	"20.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31238,
						"param_name":	"OPU_P4",
						"param_value":	"20.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31239,
						"param_name":	"I18N_CONFIG_KEY_1002331",
						"param_value":	"1.0",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	33006,
						"param_name":	"I18N_COMMON_GRID_VOLTAGE_CHARGE_REGULATION",
						"param_value":	"170",
						"unit":	"",
						"value_name":	"I18N_COMMON_ENABLE"
					}, {
						"param_addr":	33007,
						"param_name":	"UPU_V1",
						"param_value":	"215.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	33008,
						"param_name":	"UPU_V2",
						"param_value":	"207.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	33009,
						"param_name":	"UPU_V3",
						"param_value":	"207.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	33010,
						"param_name":	"UPU_V4",
						"param_value":	"207.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	33011,
						"param_name":	"UPU_P1",
						"param_value":	"0.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	33012,
						"param_name":	"UPU_P2",
						"param_value":	"80.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	33013,
						"param_name":	"UPU_P3",
						"param_value":	"80.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	33014,
						"param_name":	"UPU_P4",
						"param_value":	"80.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	33015,
						"param_name":	"I18N_CONFIG_KEY_1002461",
						"param_value":	"1.0",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	31615,
						"param_name":	"I18N_COMMON_REACTIVE_RESPONSE",
						"param_value":	"85",
						"unit":	"",
						"value_name":	"I18N_COMMON_CLOSE"
					}, {
						"param_addr":	31865,
						"param_name":	"QU_V1(AU)",
						"param_value":	"207.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	31866,
						"param_name":	"QU_V2(AU)",
						"param_value":	"220.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	31867,
						"param_name":	"QU_V3(AU)",
						"param_value":	"240.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	31868,
						"param_name":	"QU_V4(AU)",
						"param_value":	"258.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	31869,
						"param_name":	"QU_Q1(AU)",
						"param_value":	"-44.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31870,
						"param_name":	"QU_Q2(AU)",
						"param_value":	"0.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31871,
						"param_name":	"QU_Q3(AU)",
						"param_value":	"0.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	31872,
						"param_name":	"QU_Q4(AU)",
						"param_value":	"60.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	30295,
						"param_name":	"I18N_COMMON_SENIOR_SET_TEN",
						"param_value":	"170",
						"unit":	"",
						"value_name":	"I18N_COMMON_ENABLE"
					}, {
						"param_addr":	30296,
						"param_name":	"I18N_CONFIG_KEY_1001984",
						"param_value":	"258.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	30297,
						"param_name":	"I18N_COMMON_10_V_REVERT",
						"param_value":	"256.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	30800,
						"param_name":	"I18N_CONFIG_KEY_1001963",
						"param_value":	"170",
						"unit":	"",
						"value_name":	"I18N_COMMON_ENABLE"
					}, {
						"param_addr":	30801,
						"param_name":	"I18N_CONFIG_KEY_1001964",
						"param_value":	"85",
						"unit":	"",
						"value_name":	"I18N_COMMON_CLOSE"
					}, {
						"param_addr":	30799,
						"param_name":	"I18N_CONFIG_KEY_1001962",
						"param_value":	"85",
						"unit":	"",
						"value_name":	"I18N_COMMON_CLOSE"
					}, {
						"param_addr":	30798,
						"param_name":	"I18N_COMMON_LVRT_PROTECTION_SERIES",
						"param_value":	"2",
						"unit":	"",
						"value_name":	"2"
					}, {
						"param_addr":	30803,
						"param_name":	"I18N_COMMON_LVRT_VOLTAGE_PH%@1",
						"param_value":	"180.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	30804,
						"param_name":	"I18N_COMMON_LVRT_VOLTAGE_PH%@2",
						"param_value":	"70.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	30813,
						"param_name":	"I18N_COMMON_LVRT_TIME_PH%@1",
						"param_value":	"10000",
						"unit":	"ms",
						"value_name":	""
					}, {
						"param_addr":	30815,
						"param_name":	"I18N_COMMON_LVRT_TIME_PH%@2",
						"param_value":	"1000",
						"unit":	"ms",
						"value_name":	""
					}, {
						"param_addr":	30999,
						"param_name":	"I18N_CONFIG_KEY_1001971",
						"param_value":	"170",
						"unit":	"",
						"value_name":	"I18N_COMMON_ENABLE"
					}, {
						"param_addr":	31000,
						"param_name":	"I18N_CONFIG_KEY_1044",
						"param_value":	"85",
						"unit":	"",
						"value_name":	"I18N_COMMON_CLOSE"
					}, {
						"param_addr":	30998,
						"param_name":	"I18N_CONFIG_KEY_1001970",
						"param_value":	"85",
						"unit":	"",
						"value_name":	"I18N_COMMON_CLOSE"
					}, {
						"param_addr":	30997,
						"param_name":	"I18N_COMMON_HVRT_PROTECTION_SERIES",
						"param_value":	"1",
						"unit":	"",
						"value_name":	"1"
					}, {
						"param_addr":	31001,
						"param_name":	"I18N_COMMON_HVRT_VOLTAGE_PH%@1",
						"param_value":	"260.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	31012,
						"param_name":	"I18N_COMMON_HVRT_TIME_PH%@1",
						"param_value":	"1000",
						"unit":	"ms",
						"value_name":	""
					}, {
						"param_addr":	32313,
						"param_name":	"I18N_COMMON_PROTECTION_SERIES",
						"param_value":	"1",
						"unit":	"",
						"value_name":	"2"
					}, {
						"param_addr":	32322,
						"param_name":	"I18N_COMMON_UNDER_VOLTAGE_LEVEL_VALUE_PH%@1",
						"param_value":	"180.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	32323,
						"param_name":	"I18N_COMMON_OVER_VOLTAGE_LEVEL_VALUE_PH%@1",
						"param_value":	"260.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	32324,
						"param_name":	"I18N_COMMON_UNDER_FREQUENCY_LEVEL_VALUE_PH%@1",
						"param_value":	"47.00",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	32325,
						"param_name":	"I18N_COMMON_OVER_FREQUENCY_LEVEL_VALUE_PH%@1",
						"param_value":	"52.00",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	32362,
						"param_name":	"I18N_COMMON_UNDER_VOLTAGE_LEVEL_TIME_PH%@1",
						"param_value":	"10.50",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	32364,
						"param_name":	"I18N_COMMON_OVER_VOLTAGE_LEVEL_TIME_PH%@1",
						"param_value":	"1.50",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	32366,
						"param_name":	"I18N_COMMON_UNDER_FREQUENCY_LEVEL_TIME_PH%@1",
						"param_value":	"1.50",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	32368,
						"param_name":	"I18N_COMMON_OVER_FREQUENCY_LEVEL_TIME_PH%@1",
						"param_value":	"0.10",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	32326,
						"param_name":	"I18N_COMMON_UNDER_VOLTAGE_LEVEL_VALUE_PH%@2",
						"param_value":	"180.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	32327,
						"param_name":	"I18N_COMMON_OVER_VOLTAGE_LEVEL_VALUE_PH%@2",
						"param_value":	"265.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	32328,
						"param_name":	"I18N_COMMON_UNDER_FREQUENCY_LEVEL_VALUE_PH%@2",
						"param_value":	"47.00",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	32329,
						"param_name":	"I18N_COMMON_OVER_FREQUENCY_LEVEL_VALUE_PH%@2",
						"param_value":	"52.00",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	32370,
						"param_name":	"I18N_COMMON_UNDER_VOLTAGE_LEVEL_TIME_PH%@2",
						"param_value":	"1.50",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	32372,
						"param_name":	"I18N_COMMON_OVER_VOLTAGE_LEVEL_TIME_PH%@2",
						"param_value":	"0.10",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	32374,
						"param_name":	"I18N_COMMON_UNDER_FREQUENCY_LEVEL_TIME_PH%@2",
						"param_value":	"1.00",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	32376,
						"param_name":	"I18N_COMMON_OVER_FREQUENCY_LEVEL_TIME_PH%@2",
						"param_value":	"0.10",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	32318,
						"param_name":	"I18N_COMMON_OVERVOLTAGE_PROTECTION_RECOVERY_VALUE",
						"param_value":	"253.0",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	32319,
						"param_name":	"I18N_COMMON_UNDERVOLTAGE_PROTECTION_RECOVERY_VALUE",
						"param_value":	"204.9",
						"unit":	"V",
						"value_name":	""
					}, {
						"param_addr":	32320,
						"param_name":	"I18N_COMMON_OVERFREQUENCY_PROTECTION_RECOVERY_VALUE",
						"param_value":	"50.15",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	32321,
						"param_name":	"I18N_COMMON_UNDERFREQUENCY_PROTECTION_RECOVERY_VALUE",
						"param_value":	"47.50",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	32535,
						"param_name":	"I18N_COMMON_PARALLEL_CONDITION",
						"param_value":	"170",
						"unit":	"",
						"value_name":	"I18N_COMMON_ENABLE"
					}, {
						"param_addr":	32536,
						"param_name":	"I18N_COMMON_PARALLEL_FREQUENCY_LOWER_LIMIT",
						"param_value":	"47.50",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	32537,
						"param_name":	"I18N_COMMON_PARALLEL_FREQUENCY_HIGH_LIMIT",
						"param_value":	"50.15",
						"unit":	"Hz",
						"value_name":	""
					}, {
						"param_addr":	32549,
						"param_name":	"I18N_COMMON_PARALLEL_VOLTAGE_LOWER_LIMIT",
						"param_value":	"89.1",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	32550,
						"param_name":	"I18N_COMMON_PARALLEL_VOLTAGE_HIGH_LIMIT",
						"param_value":	"110.0",
						"unit":	"%",
						"value_name":	""
					}, {
						"param_addr":	32551,
						"param_name":	"I18N_COMMON_PARALLEL_DETECTION_TIME",
						"param_value":	"60",
						"unit":	"s",
						"value_name":	""
					}, {
						"param_addr":	32552,
						"param_name":	"I18N_COMMON_PARALLEL_ACTIVE_UP_RATE",
						"param_value":	"16",
						"unit":	"%",
						"value_name":	""
					}]
			}]
	}
}
```

### Energy Management Parameters

```sh-session
❯ curl 'http://10.10.10.219/device/getParam?token=4699b424-0093-45b8-b4e6-4f41a662a7e2&lang=en_us&time123456=1661832522753&dev_id=1&dev_type=35&dev_code=3343&type=9'
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"list":	[{
				"param_id":	1,
				"param_addr":	33146,
				"param_pid":	-1,
				"param_type":	1,
				"accuracy":	0,
				"param_name":	"I18N_COMMON_ENERGY_MANAGEMENT_MODE",
				"param_value":	"0",
				"unit":	"",
				"relation":	"",
				"regulation":	"",
				"range":	"",
				"options":	[{
						"name":	"I18N_COMMON_SELF_CONSUMPTION_MODE",
						"value":	"0"
					}, {
						"name":	"I18N_COMMON_FORCE_MODE_OPERATION",
						"value":	"2"
					}, {
						"name":	"I18N_COMMON_EXTERNAL_ENERGY_SCH_MODE",
						"value":	"3"
					}, {
						"name":	"I18N_COMMON_MEASURING_POIN_2",
						"value":	"4"
					}, {
						"name":	"I18N_10RT_SEPT_1527758",
						"value":	"8"
					}]
			}, {
				"param_id":	4,
				"param_addr":	33151,
				"param_pid":	-1,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_2620",
				"param_value":	"0",
				"unit":	"h",
				"relation":	"",
				"regulation":	"",
				"range":	"[0~23]",
				"options":	""
			}, {
				"param_id":	5,
				"param_addr":	33152,
				"param_pid":	-1,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_2619",
				"param_value":	"0",
				"unit":	"min",
				"relation":	"",
				"regulation":	"",
				"range":	"[0~59]",
				"options":	""
			}, {
				"param_id":	6,
				"param_addr":	33153,
				"param_pid":	-1,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_2616",
				"param_value":	"24",
				"unit":	"h",
				"relation":	"",
				"regulation":	"",
				"range":	"[0~24]",
				"options":	""
			}, {
				"param_id":	7,
				"param_addr":	33154,
				"param_pid":	-1,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_2615",
				"param_value":	"0",
				"unit":	"min&I18N_COMMON_PARAMS_SETTING_TIP",
				"relation":	"",
				"regulation":	"",
				"range":	"[0~59]",
				"options":	""
			}, {
				"param_id":	8,
				"param_addr":	33155,
				"param_pid":	-1,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_2622",
				"param_value":	"0",
				"unit":	"h",
				"relation":	"",
				"regulation":	"",
				"range":	"[0~23]",
				"options":	""
			}, {
				"param_id":	9,
				"param_addr":	33156,
				"param_pid":	-1,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_2621",
				"param_value":	"0",
				"unit":	"min",
				"relation":	"",
				"regulation":	"",
				"range":	"[0~59]",
				"options":	""
			}, {
				"param_id":	10,
				"param_addr":	33157,
				"param_pid":	-1,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_2618",
				"param_value":	"24",
				"unit":	"h",
				"relation":	"",
				"regulation":	"",
				"range":	"[0~24]",
				"options":	""
			}, {
				"param_id":	11,
				"param_addr":	33158,
				"param_pid":	-1,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_2617",
				"param_value":	"0",
				"unit":	"min&I18N_COMMON_PARAMS_SETTING_TIP",
				"relation":	"",
				"regulation":	"",
				"range":	"[0~59]",
				"options":	""
			}, {
				"param_id":	12,
				"param_addr":	33179,
				"param_pid":	-1,
				"param_type":	1,
				"accuracy":	0,
				"param_name":	"I18N_COMMON_WEEKEND_ENABLE",
				"param_value":	"170",
				"unit":	"",
				"relation":	"",
				"regulation":	"",
				"range":	"",
				"options":	[{
						"name":	"I18N_COMMON_ENABLE",
						"value":	"170"
					}, {
						"name":	"I18N_COMMON_PARA_OFF",
						"value":	"85"
					}]
			}, {
				"param_id":	13,
				"param_addr":	33180,
				"param_pid":	12,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_6186",
				"param_value":	"0",
				"unit":	"h",
				"relation":	"[170]",
				"regulation":	"",
				"range":	"[0~23]",
				"options":	""
			}, {
				"param_id":	14,
				"param_addr":	33181,
				"param_pid":	12,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_6185",
				"param_value":	"0",
				"unit":	"min",
				"relation":	"[170]",
				"regulation":	"",
				"range":	"[0~59]",
				"options":	""
			}, {
				"param_id":	15,
				"param_addr":	33182,
				"param_pid":	12,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_6182",
				"param_value":	"24",
				"unit":	"h",
				"relation":	"[170]",
				"regulation":	"",
				"range":	"[0~24]",
				"options":	""
			}, {
				"param_id":	16,
				"param_addr":	33183,
				"param_pid":	12,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_6181",
				"param_value":	"0",
				"unit":	"min&I18N_COMMON_PARAMS_SETTING_TIP",
				"relation":	"[170]",
				"regulation":	"",
				"range":	"[0~59]",
				"options":	""
			}, {
				"param_id":	17,
				"param_addr":	33184,
				"param_pid":	12,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_6188",
				"param_value":	"0",
				"unit":	"h",
				"relation":	"[170]",
				"regulation":	"",
				"range":	"[0~23]",
				"options":	""
			}, {
				"param_id":	18,
				"param_addr":	33185,
				"param_pid":	12,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_6187",
				"param_value":	"0",
				"unit":	"min",
				"relation":	"[170]",
				"regulation":	"",
				"range":	"[0~59]",
				"options":	""
			}, {
				"param_id":	19,
				"param_addr":	33186,
				"param_pid":	12,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_6184",
				"param_value":	"24",
				"unit":	"h",
				"relation":	"[170]",
				"regulation":	"",
				"range":	"[0~24]",
				"options":	""
			}, {
				"param_id":	20,
				"param_addr":	33187,
				"param_pid":	12,
				"param_type":	2,
				"accuracy":	0,
				"param_name":	"I18N_CONFIG_KEY_6183",
				"param_value":	"0",
				"unit":	"min&I18N_COMMON_PARAMS_SETTING_TIP",
				"relation":	"[170]",
				"regulation":	"",
				"range":	"[0~59]",
				"options":	""
			}, {
				"param_id":	21,
				"param_addr":	33208,
				"param_pid":	-1,
				"param_type":	1,
				"accuracy":	0,
				"param_name":	"I18N_COMMON_FORCED_CHARGE_ENABLE",
				"param_value":	"85",
				"unit":	"",
				"relation":	"",
				"regulation":	"",
				"range":	"",
				"options":	[{
						"name":	"I18N_COMMON_ENABLE",
						"value":	"170"
					}, {
						"name":	"I18N_COMMON_PARA_OFF",
						"value":	"85"
					}]
			}, {
				"param_id":	33,
				"param_addr":	33275,
				"param_pid":	-1,
				"param_type":	1,
				"accuracy":	0,
				"param_name":	"I18N_COMMON_DO_FUNCTION_CONFIG",
				"param_value":	"0",
				"unit":	"",
				"relation":	"",
				"regulation":	"",
				"range":	"",
				"options":	[{
						"name":	"I18N_COMMON_PARA_OFF",
						"value":	"0"
					}, {
						"name":	"I18N_COMMON_LOAD1_REGULATION_MODE",
						"value":	"1"
					}, {
						"name":	"I18N_COMMON_GROUND_DETECTION_ALARM",
						"value":	"2"
					}, {
						"name":	"I18N_10RT_SEPT_1527758",
						"value":	"3"
					}]
			}]
	}
}
```

## WebSocket

### Connect

```jsonc
// Request
{"lang":"en_us","token":"","service":"connect"}

// Response, includes token to use
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"service":	"connect",
		"token":	"12345678-9012-4000-0000-abcdef123456",
		"uid":	1,
		"tips_disable":	1
	}
}
```

### Ping

This is not a WebSocket ping, it's still a WebSocket text message, which the WiNet-S treats as a kind of keep-alive?

```jsonc
// Request
{"lang":"zh_cn","service":"ping","token":"","id":"cf1530ff-71e5-456a-8450-767793ba5781"}

// Response
{
	"result_code":	1,
	"result_msg":	"success"
}
```

Of note:

* the UUID in the `id` field is always random
* `lang` must be present, but doesn't have to be zh_cn, even though Web UI uses that
* `token` is always empty, and the field doesn't have to be included

### Login

```jsonc
// Request
{"lang":"en_us","token":"12345678-9012-4000-0000-abcdef123456","service":"login","passwd":"pw8888","username":"admin"}

// Response
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"service":	"login",
		"token":	"c3173fe1-380d-4406-ad54-84d77125b93a",
		"passwd":	"pw8888",
		"uid":	3,
		"role":	0,
		"tips_disable":	1
	}
}
```

### Logout

```jsonc
// Request
{"lang":"en_us","token":"c3173fe1-380d-4406-ad54-84d77125b93a","service":"logout"}

// Response
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"service":	"logout"
	}
}
```

### State

```jsonc
// Request
{"lang":"en_us","token":"12345678-9012-4000-0000-abcdef123456","service":"state"}

// Response
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"service":	"state",
		"total_fault":	"0",
		"total_alarm":	"0",
		"wireless_conn_sts":	"0",
		"wifi_conn_sts":	"0",
		"eth_conn_sts":	"1",
		"eth2_conn_sts":	"0",
		"wireless_cmd":	"170",
		"wifi_cmd":	"170",
		"cloud_conn_sts":	"1",
		"server_net_type":	"0"
	}
}
```

### Statistics

```jsonc
// Request
{"lang":"en_us","token":"12345678-9012-4000-0000-abcdef123456","service":"statistics"}

// Response
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"service":	"statistics",
		"list":	[{
				"today_energy":	"--",
				"today_energy_unit":	"kWh",
				"total_energy":	"--",
				"total_energy_unit":	"kWh",
				"curr_power":	"0.67",
				"curr_power_unit":	"kW",
				"curr_reactive":	"0.00",
				"curr_reactive_unit":	"kvar",
				"rated_power":	"5.00",
				"rated_power_unit":	"kW",
				"rated_reactive":	"3.00",
				"rated_reactive_unit":	"kvar",
				"adjust_power_uplimit":	"5.00",
				"adjust_power_uplimit_unit":	"kW",
				"adjust_reactive_uplimit":	"3.00",
				"adjust_reactive_uplimit_unit":	"kvar",
				"adjust_reactive_lowlimit":	"-3.00",
				"adjust_reactive_lowlimit_unit":	"kvar"
			}, {
				"online_num":	"2",
				"online_num_unit":	"",
				"offline_num":	"0",
				"offline_num_unit":	""
			}],
		"count":	2
	}
}
```

### Runtime

```jsonc
// Request
{"lang":"en_us","token":"12345678-9012-4000-0000-abcdef123456","service":"runtime"}

// Response
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"service":	"runtime",
		"count":	1,
		"list":	[{
				"dev_name":	"SH5.0RS(COM1-001)",
				"dev_model":	"SH5.0RS",
				"dev_type":	35,
				"dev_procotol":	2,
				"today_energy":	"--",
				"today_energy_unit":	"kWh",
				"total_energy":	"--",
				"total_energy_unit":	"kWh",
				"dev_state":	"33280",
				"dev_state_unit":	"",
				"curr_power":	"0.67",
				"curr_power_unit":	"kW",
				"reactive_power":	"0.00",
				"reactive_power_unit":	"kvar"
			}],
		"connect_count":	1,
		"off_count":	0
	}
}
```

### Device List

Unclear what `type` and `is_check_token` are for.

Response body looks pretty similar to the HTTP request. The actual devices have the same keys and values, except for the
empty `list` array when requesting over the WS.


```jsonc
// Request
{"lang":"en_us","token":"12345678-9012-4000-0000-abcdef123456","service":"devicelist","type":"0","is_check_token":"0"}

// Response
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"service":	"devicelist",
		"list":	[{
				"id":	1,
				"dev_id":	1,
				"dev_code":	3343,
				"dev_type":	35, // This appears to correspond to the `getType` HTTP request
				"dev_procotol":	2,
				"inv_type":	0,
				"dev_sn":	"REDACTED",
				"dev_name":	"SH5.0RS(COM1-001)",
				"dev_model":	"SH5.0RS",
				"port_name":	"COM1",
				"phys_addr":	"1", // This corresponds to the Modbus slave/unit ID
				"logc_addr":	"1",
				"link_status":	1,
				"init_status":	1,
				"dev_special":	"0",
				"list":	[]
			}, {
				"id":	2,
				"dev_id":	2,
				"dev_code":	8424,
				"dev_type":	44,
				"dev_procotol":	0,
				"inv_type":	0,
				"dev_sn":	"REDACTED",
				"dev_name":	"SBR128(COM1-200)",
				"dev_model":	"SBR128",
				"port_name":	"COM1",
				"phys_addr":	"200",
				"logc_addr":	"2",
				"link_status":	1,
				"init_status":	255,
				"dev_special":	"0",
				"list":	[]
			}],
		"count":	2
	}
}
```

### Realtime Values (Inverter)

Of note:

* `time123456` is not static; likely just unix timestamp, but unclear if necessary

```jsonc
// Request
{"lang":"en_us","token":"12345678-9012-4000-0000-abcdef123456","dev_id":"1","service":"real","time123456":1661762597181}

// Response
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"service":	"real",
		"list":	[{
				"data_name":	"I18N_COMMON_TOTAL_GRID_RUNNING_TIME",
				"data_value":	"--",
				"data_unit":	"h"
			}, {
				"data_name":	"I18N_COMMON_PV_DAYILY_ENERGY_GENERATION",
				"data_value":	"7.5",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_PV_TOTAL_ENERGY_GENERATION",
				"data_value":	"1473.5",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_DAILY_POWER_YIELD",
				"data_value":	"--",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_YIELD",
				"data_value":	"--",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_RUNNING_STATE",
				"data_value":	"I18N_COMMON_DISPATCH_RUN",
				"data_unit":	""
			}, {
				"data_name":	"I18N_COMMON_BUS_VOLTAGE",
				"data_value":	"379.6",
				"data_unit":	"V"
			}, {
				"data_name":	"I18N_COMMON_AIR_TEM_INSIDE_MACHINE",
				"data_value":	"25.8",
				"data_unit":	"â"
			}, {
				"data_name":	"I18N_COMMON_SQUARE_ARRAY_INSULATION_IMPEDANCE",
				"data_value":	"1107",
				"data_unit":	"kÎ©"
			}, {
				"data_name":	"I18N_CONFIG_KEY_1001188",
				"data_value":	"100.0",
				"data_unit":	"%"
			}, {
				"data_name":	"I18N_COMMON_FEED_NETWORK_TOTAL_ACTIVE_POWER",
				"data_value":	"0.00",
				"data_unit":	"kW"
			}, {
				"data_name":	"I18N_CONFIG_KEY_4060",
				"data_value":	"0.00",
				"data_unit":	"kW"
			}, {
				"data_name":	"I18N_COMMON_DAILY_FEED_NETWORK_VOLUME",
				"data_value":	"--",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_FEED_NETWORK_VOLUME",
				"data_value":	"141.1",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_ENERGY_GET_FROM_GRID_DAILY",
				"data_value":	"--",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_ELECTRIC_GRID_GET_POWER",
				"data_value":	"283.2",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_DAILY_FEED_NETWORK_PV",
				"data_value":	"0.0",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_FEED_NETWORK_PV",
				"data_value":	"129.3",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_LOAD_TOTAL_ACTIVE_POWER",
				"data_value":	"0.682",
				"data_unit":	"kW"
			}, {
				"data_name":	"I18N_COMMON_DAILY_DIRECT_CONSUMPTION_ELECTRICITY_PV",
				"data_value":	"3.8",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_DIRECT_POWER_CONSUMPTION_PV",
				"data_value":	"523.8",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_DCPOWER",
				"data_value":	"0.00",
				"data_unit":	"kW"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_ACTIVE_POWER",
				"data_value":	"0.68",
				"data_unit":	"kW"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_REACTIVE_POWER",
				"data_value":	"0.00",
				"data_unit":	"kvar"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_APPARENT_POWER",
				"data_value":	"0.68",
				"data_unit":	"kVA"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_POWER_FACTOR",
				"data_value":	"1.000",
				"data_unit":	""
			}, {
				"data_name":	"I18N_COMMON_GRID_FREQUENCY",
				"data_value":	"49.99",
				"data_unit":	"Hz"
			}, {
				"data_name":	"I18N_COMMONUA",
				"data_value":	"239.1",
				"data_unit":	"V"
			}, {
				"data_name":	"I18N_COMMON_FRAGMENT_RUN_TYPE1",
				"data_value":	"3.2",
				"data_unit":	"A"
			}, {
				"data_name":	"I18N_COMMON_PHASE_A_BACKUP_CURRENT_QFKYGING",
				"data_value":	"3.5",
				"data_unit":	"A"
			}, {
				"data_name":	"I18N_COMMON_PHASE_B_BACKUP_CURRENT_ODXCTVMS",
				"data_value":	"0.0",
				"data_unit":	"A"
			}, {
				"data_name":	"I18N_COMMON_PHASE_C_BACKUP_CURRENT_PBSQLZIX",
				"data_value":	"0.0",
				"data_unit":	"A"
			}, {
				"data_name":	"I18N_COMMON_PHASE_A_BACKUP_POWER_BRBJDGVB",
				"data_value":	"0.666",
				"data_unit":	"kW"
			}, {
				"data_name":	"I18N_COMMON_PHASE_B_BACKUP_POWER_OCDHLMZB",
				"data_value":	"0.000",
				"data_unit":	"kW"
			}, {
				"data_name":	"I18N_COMMON_PHASE_C_BACKUP_POWER_HAMBBGNL",
				"data_value":	"0.000",
				"data_unit":	"kW"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_BACKUP_POWER_WLECIVPM",
				"data_value":	"0.666",
				"data_unit":	"kW"
			}],
		"count":	36
	}
}
```

### Realtime Values (battery)

* I did not always have a separate battery device listed, until Sungrow upgraded the battery firmware remotely. There is a `real_battery` service below which uses the _inverter_ device ID.

```jsonc
// Request
// Same as inverter realtime values, but with `dev_id` of battery
{"lang":"en_us","token":"12345678-9012-4000-0000-abcdef123456","dev_id":"2","service":"real","time123456":1661762897571}

// Response
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"service":	"real",
		"list":	[{
				"data_name":	"I18N_COMMON_BATTERY_VOLTAGE",
				"data_value":	"264.3",
				"data_unit":	"V"
			}, {
				"data_name":	"I18N_COMMON_BATTERY_CURRENT",
				"data_value":	"2.6",
				"data_unit":	"A"
			}, {
				"data_name":	"I18N_COMMON_BATTERY_TEMPERATURE",
				"data_value":	"16.5",
				"data_unit":	"â"
			}, {
				"data_name":	"I18N_COMMON_REMAIN_BATTERY_POWER",
				"data_value":	"90.1",
				"data_unit":	"%"
			}, {
				"data_name":	"I18N_COMMON_BATTARY_HEALTH",
				"data_value":	"100",
				"data_unit":	"%"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_BATTERY_CHARGE",
				"data_value":	"575.9",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_BATTERY_DISCHARGE_BMS",
				"data_value":	"534.9",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_BATTERY_OPERATION_STATUS",
				"data_value":	"I18N_COMMON_STATUS_RUN",
				"data_unit":	""
			}],
		"count":	8
	}
}
```

### Battery information

* `time123456` is not static; likely just unix timestamp, but unclear if necessary

```jsonc
// Request
{"lang":"en_us","token":"12345678-9012-4000-0000-abcdef123456","dev_id":"1","service": "real_battery","time123456":1661762736979}

// Response
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"service":	"real_battery",
		"list":	[{
				"data_name":	"I18N_CONFIG_KEY_3907",
				"data_value":	"0.000",
				"data_unit":	"kW"
			}, {
				"data_name":	"I18N_CONFIG_KEY_3921",
				"data_value":	"1.068",
				"data_unit":	"kW"
			}, {
				"data_name":	"I18N_COMMON_BATTERY_VOLTAGE",
				"data_value":	"261.3",
				"data_unit":	"V"
			}, {
				"data_name":	"I18N_COMMON_BATTERY_CURRENT",
				"data_value":	"4.0",
				"data_unit":	"A"
			}, {
				"data_name":	"I18N_COMMON_BATTERY_TEMPERATURE",
				"data_value":	"16.4",
				"data_unit":	"â"
			}, {
				"data_name":	"I18N_COMMON_BATTERY_SOC",
				"data_value":	"79.5",
				"data_unit":	"%"
			}, {
				"data_name":	"I18N_COMMON_BATTARY_HEALTH",
				"data_value":	"100.0",
				"data_unit":	"%"
			}, {
				"data_name":	"I18N_COMMON_MAX_CHARGE_CURRENT_BMS",
				"data_value":	"30",
				"data_unit":	"A"
			}, {
				"data_name":	"I18N_COMMON_MAX_DISCHARGE_CURRENT_BMS",
				"data_value":	"30",
				"data_unit":	"A"
			}, {
				"data_name":	"I18N_COMMON_DAILY_BATTERY_CHARGE_PV",
				"data_value":	"3.7",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_BATTERY_CHARGE_PV",
				"data_value":	"820.4",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_DAILY_BATTERY_DISCHARGE",
				"data_value":	"7.9",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_BATTRY_DISCHARGE",
				"data_value":	"511.5",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_DAILY_BATTERY_CHARGE",
				"data_value":	"6.9",
				"data_unit":	"kWh"
			}, {
				"data_name":	"I18N_COMMON_TOTAL_BATTERY_CHARGE",
				"data_value":	"574.5",
				"data_unit":	"kWh"
			}],
		"count":	15
	}
}
```

### WiNet info

```jsonc
// Request
{"lang":"en_us","token":"12345678-9012-4000-0000-abcdef123456","service": "local"}

// Response
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"service":	"local",
		"list":	[{
				"data_name":	"I18N_COMMON_SYSTEM_TIME",
				"data_value":	"2022-08-29 18:12",
				"data_unit":	""
			}, {
				"data_name":	"I18N_COMMON_ETH_IP_ADDRESS",
				"data_value":	"10.10.10.219",
				"data_unit":	""
			}, {
				"data_name":	"I18N_COMMON_ETH_MAC_ADDRESS",
				"data_value":	"e8:68:e7:33:b6:6b",
				"data_unit":	""
			}, {
				"data_name":	"I18N_COMMON_WIFI_AP_IP_ADDRESS",
				"data_value":	"--",
				"data_unit":	""
			}, {
				"data_name":	"I18N_COMMON_WIFI_STA_IP_ADDRESS",
				"data_value":	"--",
				"data_unit":	""
			}, {
				"data_name":	"I18N_COMMON_WLAN_MAC_ADDRESS",
				"data_value":	"e8:68:e7:33:b6:68",
				"data_unit":	""
			}, {
				"data_name":	"I18N_COMMON_WIFI_SIGNAL_STRN",
				"data_value":	"--",
				"data_unit":	"dBm"
			}, {
				"data_name":	"I18N_COMMON_FTP_UPLOAD_TIME",
				"data_value":	"--",
				"data_unit":	""
			}, {
				"data_name":	"I18N_COMMON_FTP_UPLOAD_RESULT",
				"data_value":	"--",
				"data_unit":	""
			}, {
				"data_name":	"ETH1 IPV6",
				"data_value":	"--",
				"data_unit":	""
			}, {
				"data_name":	"WIFI IPV6",
				"data_value":	"--",
				"data_unit":	""
			}],
		"count":	8
	}
}
```

### Notice

This isn't requested but seems to be pushed to notify client. So far, I've only seen it to push an error that the user has been logged out due to user limit, but it may be used in other ways.

```jsonc
{
	"result_code":	100,
	"result_msg":	"normal user limit",
	"result_data":	{
		"service":	"notice"
	}
}
```

### Modbus forwarders

```jsonc
// Request
{"lang":"en_us","token":"12345678-9012-4000-0000-abcdef123456","service": "proto_modbus104"}

// Response
{
	"result_code":	1,
	"result_msg":	"success",
	"result_data":	{
		"service":	"proto_modbus104",
		"list":	[{
				"data_name":	"MODBUS-TCP IP1",
				"data_value":	"10.10.10.73",
				"data_unit":	""
			}, {
				"data_name":	"MODBUS-TCP IP2",
				"data_value":	"--",
				"data_unit":	""
			}, {
				"data_name":	"MODBUS-TCP IP3",
				"data_value":	"--",
				"data_unit":	""
			}],
		"count":	3
	}
}
```
