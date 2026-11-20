# omnect-cli

Command-line tool for developer hosts. It changes omnect OS firmware images
(device identity, certificates, config files, docker images in the image)
and works with devices in the field (SSH tunnel over a bastion host) and
Azure Device Update (create import manifest, import, remove).

## Structure

- `src/cli.rs`: clap definitions of all commands; `src/lib.rs`: `run()`
  dispatches them to the handlers.
- `src/auth.rs`, `src/config.rs`: OAuth2 PKCE login against Keycloak with a
  local redirect server; the refresh token goes to the system keyring.
- `src/device_update/`: Device Update REST client, token and blob SAS code.
- `src/file/`: image operations. `functions.rs` reads a partition out of
  the image, changes it with e2tools or mtools and writes back only the
  changed blocks. `partition.rs` reads GPT and MBR tables.
- `src/file/sparse.rs`, `bmap.rs`, `output.rs`: keep holes of sparse
  images, check and write the bmap, choose the output name and check that
  the output image and bmap do not overwrite other files.
- `src/file/signal.rs`: `RegisteredTempDir` is also removed when SIGINT,
  SIGTERM or SIGHUP stop the process.
- `src/validators/`: checks for identity config, `du-config.json` and SSH
  keys.
- `conf/`: config templates. Its `.gitignore` tracks only `*.template`;
  configs made from them are never committed.

## Constraints

- Partition numbers depend on the table type: `factory` and `cert` are 4
  and 5 on GPT, 5 and 6 on MBR. The mapping is in `get_partition_info()` in
  `src/file/functions.rs`.
- The binary calls external tools: `e2cp`, `e2mkdir`, `mcopy`, `mmd`,
  `ssh-keygen` and `docker`. The distroless image in `Dockerfile` contains
  only the executables in its explicit copy list, so a new external tool
  must be added there too.
- `CONTAINERIZED=true` (set in the docker image) changes behaviour: the
  OAuth redirect server binds `0.0.0.0:4000` instead of `127.0.0.1:4000`
  and `[::1]:4000`, `docker inject` and custom SSH config dirs are refused.
- Errors use `anyhow`; this is an application, not a library.

## Commands

- Build: `cargo build` (needs `libmagic-dev` on the host)
- Test: `cargo test`. Integration tests need e2tools and mtools on the
  host and use temp dirs under `/tmp/omnect-cli-integration-tests/`.
- Debian package: `cargo deb` (writes to `target/debian/`)
- Docker image: `./build-image.sh`. It takes the version from `Cargo.toml`
  and installs the `.deb` from `target/debian/`, so run `cargo deb` first.
