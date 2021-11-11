# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
