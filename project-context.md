<!--
  PROJECT CONTEXT — checked into git, shared across the team.

  PURPOSE: Describe what is UNIQUE to this repository. The AI agent already
  receives global rules for omnect product context, coding standards, and
  git/QA workflow via the template system. Do NOT repeat those here.

  WHAT BELONGS HERE:
    - Repo-specific architecture, entry points, and key file locations
    - Local development scripts and commands
    - Constraints or conventions specific to this repo (e.g., module layout)
    - Overrides of global rules — if this repo deviates from a global standard,
      state it here explicitly so the agent applies the correct rule.

  WHAT DOES NOT BELONG HERE:
    - omnect product context, Yocto/Azure/IoT Hub background
    - General coding standards (naming, error handling, formatting)
    - Git commit format or QA workflow rules
-->

# Project Context

## 1. Role & Responsibility

- **Role:** CLI tool to configure omnect-os firmware images (partition manipulation, identity injection, Docker container embedding) and communicate with omnect-os devices (SSH tunneling, Azure Device Update import/removal).
- **Runtime Target:** Developer workstation (native or Docker container via distroless image; `CONTAINERIZED` env var toggles container-aware behavior).

## 2. Architecture & Tech Stack

- **Language / Runtime:** Rust (edition 2024), async via Tokio + Actix-web
- **Key Frameworks:** clap 4.5 (derive) for CLI, actix-web for local OAuth2 redirect server, reqwest for HTTP
- **Notable Dependencies:**
  - Azure SDK from **omnect fork** (`omnect/azure-sdk-for-rust`) — blocked on upstream PR #1636
  - `omnect-crypto` 0.4.0 for device certificate creation
  - `gptman`/`mbrman` for partition table parsing (GPT+MBR)
  - `filemagic` for compression auto-detection
  - `keyring` for system credential storage (OAuth refresh tokens)
  - `e2tools` + `mtools` (external binaries) for ext4/FAT32 partition file operations
  - `anyhow` for error handling (CLI tool, ergonomics over type precision)

## 3. Key Entry Points & Files

- `src/main.rs` — binary entry point; sets up env_logger, calls `omnect_cli::run()`
- `src/lib.rs` — command dispatch hub; `run()` parses CLI args and delegates to handlers
- `src/cli.rs` — clap `Parser`/`Subcommand` definitions for all commands
- `src/auth.rs` — OAuth2 PKCE flow against Keycloak (local redirect server on :4000)
- `src/config.rs` — Keycloak provider config, backend URL constants
- `src/ssh.rs` — SSH tunnel creation via bastion host, ed25519 key generation
- `src/device_update.rs` — Azure Device Update import manifest creation, import/remove
- `src/docker.rs` — `docker pull --platform` + `docker save` for multi-arch images
- `src/image.rs` — firmware image architecture detection (ARM32/ARM64/x86_64)
- `src/file/mod.rs` — high-level image operations: identity config, certs, hostname patching
- `src/file/functions.rs` — partition read/modify/write via dd + e2cp/mcopy
- `src/file/partition.rs` — GPT/MBR partition table parsing
- `src/file/compression.rs` — xz/bzip2/gzip compress/decompress with auto-detection
- `src/validators/` — validation for identity config (TOML), device-update config (JSON), SSH keys
- `conf/*.template` — 11 config templates (identity, device-update, WiFi)

## 4. Repository-Specific Constraints

- External tools `e2cp`, `e2mkdir`, `mcopy`, `mmd`, `dd`, `fallocate`, `ssh-keygen`, `fdisk` must be available at runtime (Dockerfile copies them explicitly).
- Partition enum maps partition names to numbers differently for GPT vs MBR — see `file/functions.rs`.
- OAuth2 callback binds to `127.0.0.1:4000` and `[::1]:4000`; container mode overrides to `0.0.0.0`.
- `conf/` directory uses `.gitignore` to track only `*.template` files — actual configs are generated, never committed.
- Azure SDK crates use a **fork** (`omnect/azure-sdk-for-rust`) with `default-features = false`. Do not switch to upstream until PR #1636 is merged.

## 5. Local Dev Scripts

- **Build:** `cargo build` / `cargo build --release`
- **Run Tests:** `cargo test` (unit + integration; integration tests create temp dirs under `/tmp/omnect-cli-integration-tests/`)
- **Lint:** `cargo clippy` / `cargo fmt --check`
- **Debian Package:** `cargo deb` (requires `cargo-deb`; output in `target/debian/`)
- **Docker Image:** `./build-image.sh` (extracts version from Cargo.toml, builds distroless image)

## 6. Global Rule Overrides

- Uses `anyhow` instead of custom error types — this is a CLI tool where error ergonomics outweigh type precision.
