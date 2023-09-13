# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.16.1] Q3 2023
- fixed RUSTSEC-2023-0044 (explicit `cargo update`)
- fixed RUSTSEC-2023-0052 (explicit `cargo update`)
- fixed RUSTSEC-2023-0053 (explicit `cargo update`)
- fixed yanked dependency warnings (explicit `cargo update`)

## [0.16.0] Q3 2023
- added support for oauth2 authentication with keycloak
- added support for device ssh connections
- added command to inject ssh CA and device principal
- fixed code format
- fixed cargo clippy warnings

## [0.15.7] Q2 2023
- fixed e2tools issue in backend docker image

## [0.15.6] Q2 2023
- updated dependency to omnect-crypto from ssh to https
- updated readme to be open source compliant

## [0.15.5] Q2 2023
- switched to new docker registry omnectweucopsacr.azurecr.io
- fixed RUSTSEC-2023-0034 (explicit `cargo update`)

## [0.15.4] Q1 2023
- fixed GHSA-4q83-7cq4-p6wg (explicit `cargo update`)

## [0.15.3] Q1 2023
- fixed compatibility id in du-config.json.template

## [0.15.2] Q1 2023
- fixed cargo clippy warnings
- switched to anyhow based error handling
- added compatibility id to du-config.json.template

## [0.15.1] Q1 2023
- fixed misleading dps-payload.json test file

## [0.15.0] Q1 2023
- introduced command `file copy`
- removed `boot set` command

## [0.14.1] Q1 2023
- fixed "boot set" command to accept input files not named boot.scr
- updated tokio runtime to 1.23

## [0.14.0] Q4 2022
- removed "omnect-cli enrollment set -c <path>/enrollment_static.json -i <path>/image.wic" command

## [0.13.0] Q4 2022
- added support for images using a gpt layout

## [0.12.3] Q4 2022
- added option to set-identity command which allows to set a dps payload file

## [0.12.2] Q4 2022
- updated links to latest azure resources

## [0.12.1] Q4 2022
- updated du-config.json.template

## [0.12.0] Q4 2022
- renamed to omnect

## [0.11.16] Q4 2022
- set enrollment_static.conf: renamed path to /etc/omnect/

## [0.11.15] Q4 2022
- bmap file generation: bring back relative path handling

## [0.11.14] Q4 2022
- XzEncoder/XzDecoder: fixed image decoding bug of 0.11.13

## [0.11.13] Q4 2022
- bmap file generation: fixed filename
- XzEncoder/XzDecoder:
  - introduced multithreaded de-/compression
  - introduced XZ_ENCODER_PRESET <0..9> environment variable (not documented in README.md)
- cli: switched from structopt to clap

## [0.11.12] Q3 2022
- set-identity: improved error message on hostname validation error

## [0.11.11] Q3 2022
- set-identity: hostname validation against https://www.rfc-editor.org/rfc/rfc1035 in order to pass "iotedge check"

## [0.11.10] Q3 2022
- config-templates: don't configure tpm auth_key_index

## [0.11.9] Q3 2022
- adapted config templates using tpm provisioning to iotedge 1.4.x
- updated cargo dependencies
- fixed audit errors

## [0.11.8] Q3 2022
- enable ignored integration tests (possible due to fix in 0.11.4)

## [0.11.7] Q3 2022
- reorganized tests:
  - added missing integration test cases for template files in conf/ folder
  - moved unit tests from integration tests to validator tests

## [0.11.6] Q3 2022
- conf/config.toml.est.template: configure auto renewal of identity cert for dps
- added test to test current and future changes on config.toml.est.template
- updated dependency `bollard` to 0.13.*
- updated dependency `ics-dm-crypto` to 0.2.0
- Cargo.lock: implicitly updated dependencies (via `cargo update`)

## [0.11.5] Q2 2022
- updated ics-dm-crypto to version 0.1.3 (verbose on certificate validation errors)
- switched default docker registry used to pull backend image
- updated dependency `bollard` to 0.12.*

## [0.11.4] Q2 2022
- backend: enforce sparse file handling

## [0.11.3] Q2 2022
- updated ics-dm-crypto to version 0.1.2 (includes verification of certificate on
  certificate creation)

## [0.11.2] Q1 2022
- fix validator rules for config.toml files with est configured

## [0.11.1] Q1 2022
- improved readme regarding generation and usage of certificates

## [0.11.0] Q1 2022
- create and inject device cert + key for given input certificate chain + given key
- switched rust edition to 2021

## [0.10.4] Q1 2022
- fixed a bug occurring for the combination of -b option and relative image.wic path

## [0.10.3] Q1 2022
- allow additional fields in `enrollment_static.json`

## [0.10.2] Q1 2022
- backend: enforce non-empty hostname in `cargo.toml`
- updated dependencies `validator` and `regex` explicitly to fix
  RUSTSEC-2022-0013
- validator explicitly checks for empty hostname string

## [0.10.1] Q1 2022
- added new validator for connection string based provisioning in `config.toml`

## [0.10.0] Q1 2022
- added `Cargo.audit.ignore`: this file lists the `cargo audit` findings we
  ignore
- fixed dependencies to solve 2 `cargo audit` findings
- ignore dead code warnings in validator structs; structs are used at runtime
  of validator tests

## [0.9.5] Q1 2022

- subcommand `iot-hub-device-update`:
  - adapt to `iot-hub-device-update` version >= 0.8.0
  - test if input config file is a json file

## [0.9.4] Q1 2022

- remove ics_dm_first_boot.sh from factory partition, handling
  will be done during device startup
- inject wpa supplicant conf via copy_file_to_image.sh

## [0.9.3] Q4 2021

- re-enabled dd to log errors again but prevent logging statistics

## [0.9.2] Q4 2021

- fix boot.scr injection on images with active boot partition

## [0.9.1] Q4 2021

- remove async_compress and instead use xz, bzip2 and flate2 dependencies directly.
  This fixes a bug with recompression failing for xz with an lzma data error for large images
  (likely a race condition since the error did not happen every time).

## [0.9.0] Q4 2021

- added inject boot script command
- added mtools to be able to cope to boot partition

## [0.8.1] Q4 2021

- move ci pipelines out of this repo
- refactored command creation

## [0.8.0] Q4 2021

- optionally create bmap file for commands 'enrollment', 'identity',
  'iot-hub-device-update' and 'wiki'

## [0.7.0] Q4 2021

- rewrite backend scripts to use e2tool, so we don't need privileged docker context
- ics_dm_first_boot.sh is now created in factory partition
- testfiles/image.wic: added cert and factory partition
- identity, enrollment and adu config get written to factory partition
- certificates get written to cert partition

## [0.6.0] Q4 2021

- cicd: added parent pipeline to get rid of using $fly for pipeline update
- cicd: adapt to new rust builder image

## [0.5.1] Q4 2021

- fix bug where when a compressed image is used, the modifications done are not correctly written to the image.:"

## [0.5.0] Q4 2021

- use logging framework for output

## [0.4.0] Q4 2021

- added detection of image file type (to see whether it is compressed)
  - this is based on libmagic and requires libmagic-dev installed on the build system
- add transparent decompression and recompression for xz, gzip and bzip2

## [0.3.0] Q4 2021

- added validation to identity config
- cargo update to fix assertion in tokio when system clock is not monotonic (for example inside a VM)
- add /etc/hosts to rootA partition in testfiles/image.wic
- added validation to enrollment config
- applied rustfmt

## [0.2.6] Q4 2021

- backend: fix possible deadlock in finish handler

## [0.2.5] Q4 2021

- refactored command input file handling
- refactored docker_exec file bind handling

## [0.2.4] Q3 2021

- allow backend container to read/write/mknod /dev/loop-control to create new
  /dev/loop devices for lo-setup; without this fix it is possible that cli
  commands using the backend deadlock

## [0.2.3] Q3 2021

- frontend:
  - at buildtime: enable overwriting of used default backend docker registry via env var `ICS_DM_CLI_DOCKER_REG_NAME`
  - at runtime: enable overwriting of used backend docker registry via env var `ICS_DM_CLI_DOCKER_REG_NAME`
- frontend: start backend non-privileged with SYS_ADMIN capability + cgroup
  device rules to enable usage of losetup
- cicd: reuse crate.io index in test step from build step
- conf: simplified iot-identity config templates
- backend: fix permissions on /etc/aziot when injecting files into this directory
- backend: don't run docker container privileged

## [0.2.2] Q3 2021

- backend: fix setting hostname if config.toml has an inline comment
- backend: fix setting permissions for enrollment and iot-hub-device-update handling

## [0.2.1] Q3 2021

- improved backend error handling

## [0.2.0] Q3 2021

- ics-dm-ci can inject adu config
- enabled usage of parallel instances of ics-dm-cli
- pipeline doesn't have to configure backend version explicitly anymore

## [0.1.0] Q2 2021

Initial Version

- ics-dm-cli can inject wifi config into wic image file
- ics-dm-cli can inject enrollment config into wic image file
- ics-dm-cli can inject iotedge gateway configuration into wic image file
- ics-dm-cli can inject iot leaf configuration into wic image file
- ics-dm-cli can inject identity config for all device variants into wic image file
