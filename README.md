# omnect-cli
**Product page: https://www.omnect.io/home**

# Features
omnect-cli is a command-line tool to manage omnect-os empowered devices. It provides commands to inject various configurations into a flash image (wic) formerly build with [meta-omnect](https://github.com/omnect/meta-omnect). Currently the following configuration options are supported:

- Identity configuration:
  - Inject general identity configuration for AIS (Azure Identity Service)
  - Inject a device certificate and key
  - Inject bootstrap certificate for edge CA issuance/renewal
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

The package needs glibc 2.39 or later, e.g. Ubuntu 24.04 or Debian 13. On older hosts use the docker image.

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

  **Note1**: The ssh tunnel option requires some additional settings. See [here](Usage-with-docker) for more details.<br>
  **Note2**: The docker inject command is not supported by omnect-cli docker image.<br>.

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

### Inject device certificate and key for x509 based DPS provisioning and EST renewal

> **_NOTE: Use this command if your certificates are managed with [EST](https://learn.microsoft.com/en-us/azure/iot-edge/how-to-manage-device-certificates?view=iotedge-1.5&tabs=ubuntu#automatic-certificate-management-with-est-server) protocol._**

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

### Inject device certificate and key for x509 based DPS provisioning without EST management

> **_NOTE: Use this command if your certificates are NOT managed with [EST](https://learn.microsoft.com/en-us/azure/iot-edge/how-to-manage-device-certificates?view=iotedge-1.5&tabs=ubuntu#example-use-device-identity-certificate-files-from-pki-provider) protocol._**

This command:
 1. injects given device certificate pem file into a firmware image
 2. injects given device key pem file into a firmware image

Detailed description:
```sh
omnect-cli identity set-device-certificate-no-est --help
```
**Note1**: "device_id" has to match the `registration_id` respectively the `device_id` configured in `config.toml`.<br>
**Note2**: see [`config.toml.no-est.template`](conf/config.toml.no-est.template) as a corresponding `config.toml` in case of using `EST service`.

### Inject bootstrap certificate for edge CA issuance/renewal

Generates a bootstrap certificate for edge CA issuance and renewal for production over EST. Technically, this command functions similarly
`set-device-identity`:

 1. generates device specific credentials from a given intermediate certificate and key
 2. injects credentials into a firmware image

Detailed description:
```sh
omnect-cli identity set-edge-ca-certificate --help
```
**Note**: "device_id" has to match the `registration_id` respectively the `device_id` configured in `config.toml`.<br>

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

## Output image

A command that changes an image writes the result to the input image. For a compressed image it writes the uncompressed image next to it, e.g. `my-image.wic` for `my-image.wic.xz`. With `-p` (`--pack-image`) it adds the extension of the compression (`.xz`, `.bz2` or `.gzip`), e.g. `my-image.wic.xz` for `my-image.wic`. An image that already has an extension of the compression keeps its name, except that `.bzip2` becomes `.bz2`, because `bmaptool` does not read `.bzip2`. `-o` (`--output-image`) writes the result to the given path instead, and the input image stays unchanged. With `-p` its name must end in an extension of the compression that `bmaptool` reads (`.xz`, `.bz2`, `.gzip` or `.gz`), and without `-p` it must not have a compression extension.

The command fails instead of overwriting an existing file that is not the input image. To overwrite it, name it with `-o`. `-o` cannot name the input bmap, or a path that `bmaptool` reads as the bmap of an image that stays, also if it does not exist yet, e.g. `my-image.bmap` next to `my-image.xz`. An existing output image must be a regular file, so `-o` cannot name a device or a directory; to write a device, flash the image with `bmaptool copy`. It also fails before it starts if the output image or its bmap exists and is not writable for the user, or if the directory of the new file is not writable. The output image is written to a new file in its directory and then renamed, so the directory needs write access and space for one more image. An existing output image in a directory you cannot write is staged in `TMPDIR` instead and copied into it. Until the rename, the new file is in a hidden `.omnect-cli-*` directory. The command removes it when it ends or is stopped with Ctrl-C, `SIGTERM` or `SIGHUP`. It stays after a `SIGKILL` or a power loss, and you can delete it. A rename cannot replace a mount point, e.g. an image mounted with `docker run -v "$(pwd)/my-image.wic":/source/my-image.wic`, so the new image is staged in `TMPDIR` and copied into it. If this copy fails or is stopped, the new image stays, and the log names it. If the copy fails, the new bmap of `-b` stays next to it. With `-b` the command fails for such a mounted image, because the bmap would be written next to the mount point and not next to the image. Mount the directory instead. The new file belongs to the user who runs `omnect-cli`. `copy-from-image` only reads the image and writes nothing.

A command copies each partition it reads or changes to `TMPDIR` (default `/tmp`). A command that changes an image, or reads a compressed one, also copies the uncompressed image there, and for an output image that is a mount point or in a directory you cannot write also the new image. So `TMPDIR` needs space for all of them. Ctrl-C, `SIGTERM` or `SIGHUP` removes them too. If `/tmp` is a small tmpfs, set `TMPDIR` to a directory with more space.

## bmap files

Commands that change an image create a bmap file with `-b` (`--output-bmap`). It is written next to the output image as `<output image>.bmap`, e.g. `my-image.wic.bmap` for `my-image.wic` and `my-image.wic.xz.bmap` for `my-image.wic.xz`, and replaces a file with this name. An image changed in place keeps the name of its bmap: the command replaces the bmap that `bmaptool` finds for it, e.g. `my-image.wic.bmap` for `-i my-image.wic.xz -p xz`. `bmaptool` finds the bmap there without `--bmap`. The command fails if `bmaptool` would use this file also for an image that stays (`<name>` or `<name>.<compression>` next to it, for which no other bmap comes first), because it would not match that image. E.g. `my-image.wic.bmap` when `-i my-image.wic.xz` writes `my-image.wic`, also if the bmap does not exist yet, or when `-i my-image.wic` changes the image in place and `my-image.wic.xz` without its own bmap is next to it. Choose another output with `-o` then.

The bmap of the input image is the base for it: `omnect-cli` takes the file given with `--input-bmap`, else the bmap next to the image that `bmaptool` would use: `<image>.bmap`, else the same with one extension of the image removed at a time, e.g. `my-image.wic.bmap` for `my-image.wic.xz` as the Yocto build creates it. The input bmap must have the size of the image and sha256 range checksums that match it, otherwise the command fails. For an image named e.g. `my-image.bmap`, `bmaptool` would find the image itself as its bmap. Write the output image with `-o` then, and with `-b` also pass `--input-bmap` or `--no-input-bmap`. A command without `-b` fails if such a bmap exists for the output image (e.g. `my-image.wic.bmap` for `my-image.wic.xz`), because `bmaptool` would use it and it would not match the image. So when you run several commands on one image, use `-b` on each of them. `--no-input-bmap` skips a bmap next to the image that does not match, and the created bmap then maps the whole image. The created bmap maps every range of the input bmap, also when it holds only zeros, so a bmap-based flash overwrites what an earlier installation left there (partition table entries, FAT, ext4 journals). It also maps every block that the image file stores as data, which includes the blocks `omnect-cli` changed. The image file itself stays sparse.

Without an input bmap, the created bmap maps the whole image, and a flash writes every block.

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

**Note:** currently not supported via omnect-cli docker image

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

