ARG docker_namespace
ARG version_rust_container

ARG distroless_image=gcr.io/distroless/base-debian12:nonroot
FROM ${distroless_image} AS distroless

FROM ${docker_namespace}/rust:${version_rust_container} AS builder

ARG omnect_cli_version
ARG debian_dir=build/target/debian

COPY ${debian_dir}/omnect-cli_${omnect_cli_version}_amd64.deb omnect-cli_${omnect_cli_version}_amd64.deb

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    bmap-tools \
    ca-certificates \
    e2tools \
    fdisk \
    keychain \
    libc6 \
    libmagic1 \
    libssl3 \
    mtools \
        && apt-get clean && rm -rf /var/lib/apt/lists/* && \
    dpkg -i omnect-cli_${omnect_cli_version}_amd64.deb

COPY --from=distroless /var/lib/dpkg/status.d /distroless_pkgs

SHELL ["/bin/bash", "-c"]
RUN <<EOT
    set -eu

    mkdir -p /copy/status.d

    executables=( \
        /bin/bash \
        /bin/readlink \
        /bin/sed \
        /usr/bin/dd \
        /usr/bin/e2cp \
        /usr/bin/e2mkdir \
        /usr/bin/fallocate \
        /usr/bin/mcopy \
        /usr/bin/omnect-cli \
        /usr/bin/ssh-keygen \
        /usr/bin/sync \
        /usr/sbin/fdisk \
    ) 

    for executable in ${executables[@]}; do
        echo "${executable}"
        # copy executable
        mkdir -p /copy/$(dirname "${executable}")
        cp "${executable}" /copy/"${executable}"

        # gather libraries installed in distroless image to skip them
        readarray -t FILTER < <(for file in $(find /distroless_pkgs -type f -! -name "*.md5sums"); do sed -n "s/Package: \(.*\)$/\1/p" $file; done)

        # skip .so of the dynamic linker
        LOADER=$(readelf -l "${executable}" | grep "interpreter:" | sed -e "s/.*interpreter: \(.*\)]$/\1/")

        readarray -t LIBS < <(ldd "${executable}" | awk '{if ($3 == "") print $1; else print $3}')

        for LIB in ${LIBS[@]}; do
            # skip the linker loader
            if [ "$LIB" == "$LOADER" ]; then
                continue
            fi

            # the actual library location in the package may deviate from what the
            # linker specifies, so update that info and gather the package name.
            PKG_INFO=$(LOCALE=C.UTF-8 dpkg -S "*$LIB" 2> /dev/null) || continue
            PKG="${PKG_INFO%%:*}"
            LIB="${PKG_INFO##*: }"

            # skip libraries already installed in distroless
            if [[ " ${FILTER[*]} " =~ "${PKG} " ]]; then
                continue
            fi

            # copy the library and its dpkg database entries
            mkdir -p /copy/$(dirname "${LIB}")
            cp "${LIB}" /copy/"${LIB}"
            sed -n "/Package: ${PKG}/,/^$/p" /var/lib/dpkg/status > "/copy/status.d/${PKG}"
        done
    done
EOT

FROM ${distroless_image} AS base
COPY --from=builder /usr/share/misc/magic.mgc /usr/share/misc/magic.mgc
COPY --from=builder /copy/bin/ /bin/
COPY --from=builder /copy/usr/bin/ /usr/bin/
COPY --from=builder /copy/usr/sbin/ /usr/sbin/
COPY --from=builder /copy/usr/lib/ /usr/lib/
COPY --from=builder /copy/lib/ /lib/
COPY --from=builder /copy/status.d /var/lib/dpkg/status.d

ENTRYPOINT [ "/usr/bin/omnect-cli" ]