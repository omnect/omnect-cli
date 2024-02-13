docker build \
  --build-arg DOCKER_NAMESPACE=omnectweucopsacr.azurecr.io \
  --build-arg VERSION_RUST_CONTAINER=1.75.0-bookworm \
  --build-arg omnect_cli_version=0.20.1 \
  --build-arg debian_dir=target/debian \
  -f Dockerfile \
  --progress=plain \
  -t omnect-cli:0.20.1 .