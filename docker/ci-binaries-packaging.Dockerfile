# syntax=docker/dockerfile:1
# ---------------------------------------------------------------------------------------------------------
# This Dockerfile is intended to be built as part of matrixon's CI pipeline.
# It does not build matrixon in Docker, but just copies the matching build artifact from the build jobs.
#
# It is mostly based on the normal matrixon Dockerfile, but adjusted in a few places to maximise caching.
# Credit's for the original Dockerfile: Weasy666.
# ---------------------------------------------------------------------------------------------------------

FROM docker.io/alpine:3.16.0@sha256:4ff3ca91275773af45cb4b0834e12b7eb47d1c18f770a0b151381cd227f4c253 AS runner


# Standard port on which matrixon launches.
# You still need to map the port when using the docker command or docker-compose.
EXPOSE 6167

# Users are expected to mount a volume to this directory:
ARG DEFAULT_DB_PATH=/var/lib/matrix-matrixon

ENV matrixon_PORT=6167 \
    matrixon_ADDRESS="0.0.0.0" \
    matrixon_DATABASE_PATH=${DEFAULT_DB_PATH} \
    matrixon_CONFIG=''
#    └─> Set no config file to do all configuration with env vars

# matrixon needs:
#   ca-certificates: for https
#   iproute2: for `ss` for the healthcheck script
RUN apk add --no-cache \
    ca-certificates \
    iproute2

ARG CREATED
ARG VERSION
ARG GIT_REF
# Labels according to https://github.com/opencontainers/image-spec/blob/master/annotations.md
# including a custom label specifying the build command
LABEL org.opencontainers.image.created=${CREATED} \
    org.opencontainers.image.authors="matrixon Contributors" \
    org.opencontainers.image.title="matrixon" \
    org.opencontainers.image.version=${VERSION} \
    org.opencontainers.image.vendor="matrixon Contributors" \
    org.opencontainers.image.description="A Matrix NextServer written in Rust" \
    org.opencontainers.image.url="https://matrixon.rs/" \
    org.opencontainers.image.revision=${GIT_REF} \
    org.opencontainers.image.source="https://gitlab.com/famedly/matrixon.git" \
    org.opencontainers.image.licenses="Apache-2.0" \
    org.opencontainers.image.documentation="https://gitlab.com/famedly/matrixon" \
    org.opencontainers.image.ref.name=""


# Test if matrixon is still alive, uses the same endpoint as Element
COPY ./docker/healthcheck.sh /srv/matrixon/healthcheck.sh
HEALTHCHECK --start-period=5s --interval=5s CMD ./healthcheck.sh

# Improve security: Don't run stuff as root, that does not need to run as root:
# Most distros also use 1000:1000 for the first real user, so this should resolve volume mounting problems.
ARG USER_ID=1000
ARG GROUP_ID=1000
RUN set -x ; \
    deluser --remove-home www-data ; \
    addgroup -S -g ${GROUP_ID} matrixon 2>/dev/null ; \
    adduser -S -u ${USER_ID} -D -H -h /srv/matrixon -G matrixon -g matrixon matrixon 2>/dev/null ; \
    addgroup matrixon matrixon 2>/dev/null && exit 0 ; exit 1

# Change ownership of matrixon files to matrixon user and group
RUN chown -cR matrixon:matrixon /srv/matrixon && \
    chmod +x /srv/matrixon/healthcheck.sh && \
    mkdir -p ${DEFAULT_DB_PATH} && \
    chown -cR matrixon:matrixon ${DEFAULT_DB_PATH}

# Change user to matrixon
USER matrixon
# Set container home directory
WORKDIR /srv/matrixon

# Run matrixon and print backtraces on panics
ENV RUST_BACKTRACE=1
ENTRYPOINT [ "/srv/matrixon/matrixon" ]

# Depending on the target platform (e.g. "linux/arm/v7", "linux/arm64/v8", or "linux/amd64")
# copy the matching binary into this docker image
ARG TARGETPLATFORM
COPY --chown=matrixon:matrixon ./$TARGETPLATFORM /srv/matrixon/matrixon
