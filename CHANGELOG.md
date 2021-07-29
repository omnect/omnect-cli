# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
