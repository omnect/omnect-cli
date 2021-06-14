# ics-dm-cli

# Troubleshooting
## No credential store support
`ics-dm-cli` needs to pull a docker image "todo" as backend for some cli
commands. If you use a docker environment with credential store you have to
pull the image prior to calling `ics-dm-cli` manually.
```sh
docker pull "todo"
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

Copyright (c) 2021 conplement AG
