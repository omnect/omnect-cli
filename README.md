# omnect-cli
**Product page: https://www.omnect.io/home**

# Features
omnect-cli is a cli tool to manage your omnect-devices. It provides commands to inject various configurations into a flash image (wic) build with [meta-omnect](https://github.com/omnect/meta-omnect). Currently the following configurations options are supported:
- Wifi configuration
  - Inject wifi configuration via wpa_supplicant into all omnect-device variants
- Identity configuration
  - Inject general identity configuration for AIS (Azure Identity Service)
  - Inject an iotedge gateway identity configuration for AIS
  - Inject an iot leaf identity configuration for AIS
  - Inject a device certificate with corresponding key from a given intermediate full-chain-certificate and corresponding key
  - Inject an ssh root ca and device principal for ssh tunnel creation.
- Device Update for IoT Hub configuration
  - Inject [`du-config.json`](https://docs.microsoft.com/en-us/azure/iot-hub-device-update/device-update-configuration-file)
- Boot configuration
  - Inject `boot.scr`
- File
  - Copy a file into the image, restricted to partitions boot, cert, factory
- SSH
  - Open an ssh tunnel on a device in the field to connect to it.

# Prerequisites

Depending on your intended use use you want to install the following packages.

- openssh: For the `ssh` command.
- docker: For the `file`, `iot-hub-device-update`, `identity` and `wifi` commands.

# Download prebuild Docker image
- login to azure docker registry either via admin user
    ```sh
    docker login omnectweucopsacr.azurecr.io
    ```
    or via your AAD account
    ```sh
    az login
    az acr login -n omnectweucopsacr
    ```
- Pull latest docker image
    ```sh
    docker pull omnectweucopsacr.azurecr.io/omnect-cli-backend:latest
    ```
If you want to use a specific version, look for available versions in the [registry](https://portal.azure.com/#@conplementag2.onmicrosoft.com/resource/subscriptions/ebaba6ec-e467-4409-b73e-bdd5e34a34c1/resourceGroups/omnect-weu-cops-acrrg/providers/Microsoft.ContainerRegistry/registries/omnectweucopsacr/overview).

# Installation
Ensure ~/bin/ exists and is in your $PATH before executing:

```sh
docker run --rm --entrypoint cat omnectweucopsacr.azurecr.io/omnect-cli-backend:latest /install/omnect-cli > ~/bin/omnect-cli && chmod +x ~/bin/omnect-cli
```

# Wifi configuration
## Inject wifi configuration
Adapt either [wpa_supplicant.conf.simple.template](conf/wpa_supplicant.conf.simple.template) or [wpa_supplicant.conf.template](conf/wpa_supplicant.conf.template).
Use `wpa_passphrase` to generate your `psk`. Depending on your host system you may have to install `wpa_supplicant` to be able to use `wpa_passphrase`.

```sh
omnect-cli wifi set -c <path>/wpa_supplicant.conf -i <path>/image.wic

Options:
  -b create bmap file
```

# Identity configuration
## Configuration of devices NOT part of a gateway with leaf scenario
For `omnect-iot-devices` and `omnect-iotedge-devices` adapt [config.toml.est.template](conf/config.toml.est.template) or [config.toml.tpm.template](conf/config.toml.tpm.template) to your needs.

```sh
omnect-cli identity set-config -c <path>/config.toml -i <path>/image.wic

Options:
  -p <path>/payload.json
  -b create bmap file
```
For further information on using dps payloads read the following [link](https://learn.microsoft.com/de-de/azure/iot-dps/concepts-custom-allocation).

## Prepare `omnect-iotedge-gateway-device` and `omnect-iot-leaf-device` for a transparent gateway with leaf scenario
Follow this article [Configure an IoT Edge device to act as a transparent gateway](https://docs.microsoft.com/en-us/azure/iot-edge/how-to-create-transparent-gateway?view=iotedge-2020-11) to understand the iotedge based transparent gateway setup. We assume that you use a X.509 CA certificate setup.

## Gateway configuration
### Certificates
Follow the article [Create demo certificates to test IoT Edge device features](https://docs.microsoft.com/en-us/azure/iot-edge/how-to-create-test-certificates?view=iotedge-2020-11=) in case you don't have a workflow for certificate creation yet.
For the gateway, you need the following files:
  - `azure-iot-test-only.root.ca.cert.pem`
  - `iot-edge-ca-<name>-full-chain.cert.pem`
  - `iot-edge-ca-<name>.key.pem`

### Config file
Adapt [config.toml.gateway.est.template](conf/config.toml.gateway.est.template) or [config.toml.gateway.tpm.template](conf/config.toml.gateway.tpm.template) to your needs.

### Inject configuration
```sh
omnect-cli identity  set-iotedge-gateway-config -c <path>/iotedge_config.toml -i <path>/iotedge_image.wic  -r <path>/azure-iot-test-only.root.ca.cert.pem -d <path>/iot-edge-device-ca-<name>-full-chain.cert.pem -k <path>/iot-edge-device-ca-<name>.key.pem

Options:
  -b create bmap file
```

## Leaf configuration
### Restriction
We provision iot applications as modules. Currently our leaf device configuration is restricted to SaS authentication. See https://azure.github.io/iot-identity-service/develop-an-agent.html#connecting-your-agent-to-iot-hub for details.

### Certificates
For the leaf device with SaS provisioning you only need the root ca certificate:
  - `azure-iot-test-only.root.ca.cert.pem`

### Config file
Adapt [config.toml.iot-leaf.template](conf/config.toml.iot-leaf.template) to your needs.

### Inject configuration
```sh
omnect-cli identity set-iot-leaf-sas-config -c <path>/iot_config.toml -i <path>/leaf_image.wic  -r <path>/azure-iot-test-only.root.ca.cert.pem

Options:
  -b create bmap file
```

## Device certificate and key

For a given full-chain intermediate certificate and corresponding key, both as pem files, generate a device certificate and device key valid for 365 days.
```sh
omnect-cli identity set-device-certificate -d "device_id" -i <path>/image.wic -c <path>/intermediate_full_chain_cert.pem -k <path>/intermediate_cert_key.pem -D 365
```
Note: "device_id" has to match the `registration_id` respectively the `device_id` configured in `config.toml`.

See [`config.toml.est.template`](conf/config.toml.est.template) as a corresponding `config.toml` is case of using `EST service`.

### Get full-chain intermediate certificate and key for existing OMNECT PKI
Please get into contact with us in case you want to use our existing cloud services for device provisioning. We can provide certificate and key file to configure your device.

### Generate your own full-chain intermediate certificate and key
In case you intend to use your own certificates (e.g. because you want to use your own `PKI` and/or `EST service`), you can find some information about generating certificate and key here: https://docs.microsoft.com/en-us/azure/iot-edge/how-to-create-test-certificates?view=iotedge-2020-11.

## SSH Tunnel configuration

For the ssh feature, the device requires the public key of the ssh root ca and the principal. The latter should be the device id.
```sh
omnect-cli identity set-ssh-tunnel-certificate --image <path>/image.wic --root_ca <path>/ssh_ca.pub --device-principal "device_id"
```

# Device Update for IoT Hub configuration
## Inject `du-config.json`

```sh
omnect-cli iot-hub-device-update set -c <path>/du-config.json -i <path>/image.wic

Options:
  -b create bmap file
```

# Copy Files into Image

Copying files into the image is restricted to partions `boot`, `cert` and `factory`.

Note: If you need special permissions on copied files, you have to additionally copy a systemd-tmpfiles.d configuration file which handles these permissions.

## Inject `boot.scr`

```sh
omnect-cli file copy -f <path>/boot.scr -i <path>/image.wic -p boot -d /boot.scr

Options:
  -b create bmap file
```

## Inject iptables configuration
```sh
omnect-cli file copy -f <path>/iptables.rules -i <path>/image.wic -p factory -d /etc/iptables/iptables.rules

Options:
  -b create bmap file
```

## Inject systemd-tmpfiles.d configuration
```sh
omnect-cli file copy -f <path>/my_custom_tmpfilesd.conf -i <path>/image.wic -p factory -d /etc/tmpfiles.d/my_custom_tmpfilesd.conf

Options:
  -b create bmap file
```

# Creating an SSH Tunnel

One can use `omnect-cli` to create a tunneled SSH connection to a device in the field. This is especially useful if the device is behind a NAT and can not directly be contacted. The device must have the `ssh` activated for this. Per default, this command will create a single use ssh key pair, certificate, and ssh configuration to establish a connection to the device.

Note: if unused, the tunnel will close after 5 minutes.

Creating the ssh tunnel:
```sh
omnect-cli ssh <device>

Options:
  -u <name> optional: name of the user on the device
  -d <dir> optional: directory where the ssh key pair, certificate, and configuration are stored to
  -k <key> optional: path to an existing private ssh key to use for the connection. Requires the existance of the public key <key>.pub
  -c <config_path> optional: path where the ssh configuration should be stored to
```

## Example Usage

Open an ssh tunnel to the device `test_device` as follows:
```sh
~ omnect-cli ssh test_device

Successfully established ssh tunnel!
Certificate dir: /run/user/1000/omnect-cli
Configuration path: /run/user/1000/omnect-cli/ssh_config
Use the configuration in "/run/user/1000/omnect-cli/ssh_config" to use the tunnel, e.g.:
ssh -F /run/user/1000/omnect-cli/ssh_config test_device
```
Now follow the command output to establish a connection to the device as such:

```sh
~ ssh -F /run/user/1000/omnect-cli/ssh_config test_device

[omnect@test_device ~]$
```

# Troubleshooting

If anything goes wrong, setting RUST_LOG=debug enables output of debug information.

## No credential store support
`omnect-cli` needs to pull a docker image `omnectweucopsacr.azurecr.io/omnect-cli-backend` as backend for some cli
commands. If you use a docker environment with credential store you have to
pull the image prior to calling `omnect-cli` manually. (Note this is not necessary if you installed ´omnect-cli´ via: [Installation](#installation))
```sh
docker pull omnectweucopsacr.azurecr.io/omnect-cli-backend:$(omnect-cli --version | awk '{print $2}')
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
- [ ] Describe local build and overwriting backend docker registry via `OMNECT_CLI_DOCKER_REG_NAME`
      at buildtime and at runtime
- [ ] replace link to internal wiki and describe omnect image variants

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

---

copyright (c) 2021 conplement AG<br>
Content published under the Apache License Version 2.0 or MIT license, are marked as such. They may be used in accordance with the stated license conditions.

