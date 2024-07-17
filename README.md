# omnect-cli
**Product page: https://www.omnect.io/home**

# Features
omnect-cli is a command-line tool to manage omnect-os empowered devices. It provides commands to inject various configurations into a flash image (wic) formerly build with [meta-omnect](https://github.com/omnect/meta-omnect). Currently the following configuration options are supported:

- Identity configuration:
  - Inject general identity configuration for AIS (Azure Identity Service)
  - Inject a device certificate with corresponding key from a given intermediate full-chain-certificate and corresponding key
- Device Update for IoT Hub:
  - manage updates (create, import, remove) (https://learn.microsoft.com/en-us/azure/iot-hub-device-update/import-concepts)
  - inject configuration file `du-config.json` (https://docs.microsoft.com/en-us/azure/iot-hub-device-update/device-update-configuration-file)
- Generic configuration of services
  - copy files to image in order to configure e.g. boot service, firewall, wifi and others
  - copy files from image, e.g. to patch and re-inject configurations
- ssh:
  - inject a ssh root ca for ssh tunnel creation
- docker:
  - inject packed docker images into the image

Further omnect-cli supports device management features. Currently supported:
  - open a ssh tunnel on a device in the field to connect to it

# Installation
## Debian package

Available debian packages can be listed as a xml document via this [link](https://omnectassetst.blob.core.windows.net/omnect-cli?restype=container&comp=list). Choose, download and install a version:
```sh
wget https://omnectassetst.blob.core.windows.net/omnect-cli/omnect-cli_<version>_amd64.deb
sudo dpkg -i omnect-cli_<version>_amd64.deb
```
**Note**: `dpkg` lists necessary runtime dependencies in case they are not present.

## Docker image

`omnect-cli` is also provided as docker image.<br>

Example usage:

```ssh
docker run --rm -it \
  -v "$(pwd)":/source \
  -e RUST_LOG=debug \
  -u $(id -u) \
  omnect/omnect-cli:latest file copy-to-image --files /source/my-source-file,boot:/my-dest-file -i /source/my-image.wic
  ```

  **Note1**: `-b` option to create bmap file is not supported by docker image.<br>
  **Note2**: The ssh tunnel option requires some additional settings. See [here](Usage-with-docker) for more details.

# Build from sources

The application can be built via `cargo` as usual. A prerequisite is libmagic, e.g. the package libmagic-dev must be installed on a debian-based host system.

# Commands
## Identity configuration
### Inject identity

This command injects an [Azure IoT Identity](https://azure.github.io/iot-identity-service/) configuration into a firmware image.

Detailed description:
```sh
omnect-cli identity set-config --help
```

**Note1**: For `omnect-iotedge-devices` adapt [config.toml.est.template](conf/config.toml.est.template) or [config.toml.tpm.template](conf/config.toml.tpm.template) to your needs.<br>
**Note2**: For further information on using dps payloads read the following [link](https://learn.microsoft.com/de-de/azure/iot-dps/concepts-custom-allocation).

### Inject device certificate and key for x509 based DPS provisioning

This command:
 1. generates device specific credentials from a given intermediate certificate and key
 2. injects credentials into a firmware image

Detailed description:
```sh
omnect-cli identity set-device-certificate --help
```
**Note1**: "device_id" has to match the `registration_id` respectively the `device_id` configured in `config.toml`.<br>
**Note2**: see [`config.toml.est.template`](conf/config.toml.est.template) as a corresponding `config.toml` in case of using `EST service`.

#### Get full-chain intermediate certificate and key for existing OMNECT PKI
Please get into contact with us in case you want to use our existing cloud services for device provisioning. We can provide certificate and key file to configure your device.

#### Generate your own full-chain intermediate certificate and key
In case you intend to use your own certificates (e.g. because you want to use your own `PKI` and/or `EST service`), you can find some information about generating certificate and key here: https://docs.microsoft.com/en-us/azure/iot-edge/how-to-create-test-certificates?view=iotedge-2020-11.

## Device Update for IoT Hub
### Create import manifest
This command creates the device update import manifest which is used later by the `import-update` command.

Detailed description:
```sh
omnect-cli iot-hub-device-update create-import-manifest --help
```

### Import update to IoT Hub
This command imports an update into Azure Device Update for IoT Hub by providing a import manifest formerly created by `create-import-manifest` command.

Detailed description:
```sh
omnect-cli iot-hub-device-update import-update --help
```

**Note**: The import process may take several minutes.

### Remove update from IoT Hub
This command removes an update from Azure Device Update for IoT Hub that was previously imported by `import-update` command.

Detailed description:
```sh
omnect-cli iot-hub-device-update remove-update --help
```

### Inject `du-config.json` configuration file
This command injects a device update configuration into a firmware image.

Detailed description:
```sh
omnect-cli iot-hub-device-update set-device-config --help
```

## Copy files

Copying files into or from the image is restricted to partitions `boot`, `rootA`, `cert` and `factory`. Destination paths that are not existing will be created on host as well as on image.

### Copy files from image

`omnect-cli` allows copying multiple files from multiple partitions in one command:

Detailed description:
```sh
omnect-cli file copy-from-image --help
```

### Copy files to image

`omnect-cli` allows copying multiple files to multiple partitions in one command:

Detailed description:
```sh
omnect-cli file copy-to-image --help
```

**Note1**: If you need special permissions on copied files, you have to additionally copy a systemd-tmpfiles.d configuration file which handles these permissions.<br>
**Note2**: Injecting files allows configuration of device behavior and services, e.g.:
- Boot: inject `boot.scr` or grub.cfg
- Firewall: inject `iptables.rules`
- File permissions: inject `systemd-tmpfiles.d`
- Wifi: inject `wpa_supplicant-wlan0.conf`

## ssh tunnel

### Inject ssh tunnel credentials

For the ssh feature, the device requires the public key of the ssh root ca.

Detailed description:
```sh
omnect-cli ssh set-certificate --help
```

### Creating a ssh tunnel

One can use `omnect-cli` to create a tunneled ssh connection to a device in the field. This is especially useful if the device is behind a NAT and can not directly be contacted. The device must have the `ssh` activated for this. Per default, this command will create a single use ssh key pair, certificate, and ssh configuration to establish a connection to the device.

To create an ssh tunnel, `omnect-cli` must first authenticate against the authentication service. The service credentials vary, depending on the omnect cloud environment. They default to omnect-prod.

**Note**: if unused, the tunnel will close after 5 minutes.

Detailed description:
```sh
omnect-cli ssh set-connection --help
```

#### Example usage

Open an ssh tunnel to the device `prod_device` in the `prod` environment as follows:
```sh
omnect-cli ssh set-connection prod_device

Successfully established ssh tunnel!
Certificate dir: /run/user/1000/omnect-cli
Configuration path: /run/user/1000/omnect-cli/config
Use the configuration in "/run/user/1000/omnect-cli/config" to use the tunnel, e.g.:
ssh -F /run/user/1000/omnect-cli/config prod_device
```
Now follow the command output to establish a connection to the device as such:

```sh
ssh -F /run/user/1000/omnect-cli/config prod_device

[omnect@prod_device ~]$
```

To connect to the device `dev_device` in the `dev` environment, we additionally
have to supply a configuration with backend and the authentication details for
the `dev` environment:

```dev_env.toml
backend = 'https://cp.dev.omnect.conplement.cloud'

[auth.Keycloak]
provider = 'https://keycloak.omnect.conplement.cloud'
realm = 'cp-dev'
client_id = 'cp-cli'
bind_addrs = ['127.0.0.1:4000', '[::1]:4000']
redirect = 'http://localhost:4000'
```

You then have to pass this configuration with the `--env` flag:
```sh
omnect-cli ssh set-connection dev_device --env dev_env.toml

Successfully established ssh tunnel!
...
```

#### Usage with docker

To use the ssh tunnel feature within a docker image, some additional steps are
necessary:

1. bind mount the directory to where the ssh keys and configurations should be generated to
2. set `CONTAINERIZED` environment variable as "true". The provided docker image
   already has this variable set accordingly. **Note**: if running on a Windows
   host, you additionally have to set the `CONTAINER_HOST` variable to
   `windows`.
3. map the container's port on localhost 4000 to the hosts port 4000

With our `dev_device` from above, the call on a Windows host would then look,
for example, as follows:

```sh
docker run --rm \
  -u 0:0 \
  -v "C:/absolute/host/path/to/.ssh":/root/.config/omnect-cli \
  -v dev_env.toml:/dev_env.toml \
  -e CONTAINER_HOST=windows \
  -p 127.0.0.1:4000:4000 \
  omnect/omnect-cli:latest \
  ssh set-connection dev_device --env dev_env.toml
```

If you want to use a custom backend configuration, you additionally have to
bind mount the config file, as well, i.e., `-v host/path/to/config.toml:/config.toml`,
and then tell omnect-cli to use this path.

## docker

### Inject docker images into firmware images

This command downloads and injects a packed docker image into a firmware image.

Detailed description:
```sh
omnect-cli docker inject --help
```

# Troubleshooting

If anything goes wrong, setting RUST_LOG=debug enables output of debug information.

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

