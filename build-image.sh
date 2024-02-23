# local build
cli_version=$(toml get --raw Cargo.toml package.version)
rust_version="1.76.0-bookworm"

docker build \
  --build-arg docker_namespace=omnectweucopsacr.azurecr.io \
  --build-arg version_rust_container="${rust_version}" \
  --build-arg omnect_cli_version="${cli_version}" \
  --build-arg debian_dir=target/debian \
  -f Dockerfile \
  --progress=plain \
  -t omnect-cli:"${cli_version}" .