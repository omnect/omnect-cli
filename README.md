# ics-dm-cli
ics-dm-cli is a cli tool to manage your ics-dm-device [variants](https://wiki.conplement.de/display/ICSDeviceManagement/ICS_DeviceManagement+Home).

# Features
ics-dm-cli provides commands to inject various configurations into a flash image (wic) build with [meta-ics-dm](https://github.com/ICS-DeviceManagement/meta-ics-dm). Currently the following configurations options are supported:
- Wifi configuration
  - Inject wifi configuration via wpa_supplicant into all ics-dm-device variants
- Enrollment demo configuration
  - Inject [enrollment demo](https://github.com/ICS-DeviceManagement/enrollment) configuration
- Identity configuration
  - Inject general identity configuration for AIS (Azure Identity Service)
  - Inject an iotedge gateway identity configuration for AIS
  - Inject an iotedge leaf identity configuration for AIS
- Device Update for IoT Hub configuration
  - Inject `aduconf.txt`

# Download prebuild Docker image
- login to azure docker registry either via admin user
    ```sh
    docker login icsdm.azurecr.io
    ```
    or via your AAD account
    ```sh
    az login
    az acr login -n icsdm
    ```
- Pull latest docker image
    ```sh
    docker pull icsdm.azurecr.io/ics-dm-cli-backend:latest
    ```
If you want to use a specific version, look for available versions in the [registry](https://portal.azure.com/#@CONPLEMENTAG1.onmicrosoft.com/resource/subscriptions/ff939028-597d-472b-a7cc-bca2ac8f96bd/resourcegroups/DockerRegistry/providers/Microsoft.ContainerRegistry/registries/icsdm/repository).

# Installation
Ensure ~/bin/ exists and is in your $PATH before executing:

```sh
docker run --rm --entrypoint cat icsdm.azurecr.io/ics-dm-cli-backend:latest /install/ics-dm-cli > ~/bin/ics-dm-cli && chmod +x ~/bin/ics-dm-cli
```

# Wifi configuration
## Inject wifi configuration
Adapt either [wpa_supplicant.conf.simple.template](conf/wpa_supplicant.conf.simple.template) or [wpa_supplicant.conf.template](conf/wpa_supplicant.conf.template).
Use `wpa_passphrase` to generate your `psk`. Depending on your host system you may have to install `wpa_supplicant` to be able to use `wpa_passphrase`.

```sh
ics-dm-cli wifi set -c <path>/wpa_supplicant.conf -i <path>/image.wic
```

# Enrollment configuration
## Inject enrollment configuration
Adapt [enrollment_static.conf.template](conf/enrollment_static.conf.template) to your needs.

```sh
ics-dm-cli enrollment set -e <path>/enrollment_static.conf -i <path>/image.wic
```

# Identity configuration
## Configuration of devices NOT part of a gateway with leaf scenario
For `ics-iot-devices` and `ics-iotedge-devices` adapt [config.toml.ics-iot.template](conf/config.toml.ics-iot.template) or [config.toml.ics-iotedge.template](conf/config.toml.ics-iotedge.template) to your needs.

```sh
ics-dm-cli identity set-config -c <path>/config.toml -i <path>/image.wic
```

## Prepare `ics-iotedge-gateway-device` and `ics-iot-leaf-device` for a transparent gateway with leaf scenario
Follow this article [Configure an IoT Edge device to act as a transparent gateway](https://docs.microsoft.com/en-us/azure/iot-edge/how-to-create-transparent-gateway?view=iotedge-2020-11) to understand the iotedge based transparent gateway setup. We assume that you use a X.509 CA certificate setup.

## Gateway configuration
### Certificates
Follow the article [Create demo certificates to test IoT Edge device features](https://docs.microsoft.com/en-us/azure/iot-edge/how-to-create-test-certificates?view=iotedge-2020-11=) in case you don't have a workflow for certificate creation yet.
For the gateway, you need the following files:
  - `azure-iot-test-only.root.ca.cert.pem`
  - `iot-edge-ca-<name>-full-chain.cert.pem`
  - `iot-edge-ca-<name>.key.pem`

### Config file
Adapt [config.toml.ics-iotedge-gateway.template](conf/config.toml.ics-iotedge-gateway.template) to your needs.

### Inject configuration
```sh
ics-dm-cli identity  set-iotedge-gateway-config -c <path>/iotedge_config.toml -i <path>/iotedge_image.wic  -r <path>/azure-iot-test-only.root.ca.cert.pem -d <path>/iot-edge-device-ca-<name>-full-chain.cert.pem -k <path>/iot-edge-device-ca-<name>.key.pem
```

## Leaf configuration
### Restriction
We provision iot applications as modules. Currently our leaf device configuration is restricted to SaS authentication. See https://azure.github.io/iot-identity-service/develop-an-agent.html#connecting-your-agent-to-iot-hub for details.

### Certificates
For the leaf device with SaS provisioning you only need the root ca certificate:
  - `azure-iot-test-only.root.ca.cert.pem`

### Config file
Adapt [config.toml.ics-iot-leaf.template](conf/config.toml.ics-iot-leaf.template) to your needs.

### Inject configuration
```sh
ics-dm-cli identity set-iot-leaf-sas-config -c <path>/iot_config.toml -i <path>/leaf_image.wic  -r <path>/azure-iot-test-only.root.ca.cert.pem
```

# Device Update for IoT Hub configuration
## Inject `adu-conf.txt`

```sh
ics-dm-cli iot-hub-device-update set -c <path>/adu-conf.txt -i <path>/image.wic
```


# Troubleshooting
## No credential store support
`ics-dm-cli` needs to pull a docker image `icsdm.azurecr.io/ics-dm-cli-backend` as backend for some cli
commands. If you use a docker environment with credential store you have to
pull the image prior to calling `ics-dm-cli` manually. (Note this is not necessary if you installed ´ics-dm-cli´ via: [Installation](#installation))
```sh
docker pull icsdm.azurecr.io/ics-dm-cli-backend:$(ics-dm-cli --version | awk '{print $2}')
```

## Verify configuration is functional
Check for valid AIS identity configuration on iotedge devices:
```sh
iotedge system logs
```

Check for valid AIS identity configuration on iot devices:
```sh
aziotctl system logs
```

Check for valid wifi configuration:
```sh
systemctl status wpa_supplicant@wlan0
```

# ToDo's
- [ ] Make linked ICS_DeviceManagement Wiki public

# License
Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

# Contribution
Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

Copyright (c) 2021 conplement AG
