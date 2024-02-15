docker build \
  --build-arg docker_namespace=omnectweucopsacr.azurecr.io \
  --build-arg version_rust_container=1.76.0-bookworm \
  --build-arg omnect_cli_version=0.20.1 \
  --build-arg debian_dir=target/debian \
  -f Dockerfile \
  --progress=plain \
  -t omnect-cli:0.20.1 .