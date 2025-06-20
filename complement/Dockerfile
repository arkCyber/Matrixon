FROM rust:1.81.0

WORKDIR /workdir

RUN apt-get update && apt-get install -y --no-install-recommends \
    libclang-dev

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY src src
RUN cargo build --release \
    && mv target/release/matrixon matrixon \
    && rm -rf target

# Install caddy
RUN apt-get update \
    && apt-get install -y \
    debian-keyring \
    debian-archive-keyring \
    apt-transport-https \
    curl \
    && curl -1sLf 'https://dl.cloudsmith.io/public/caddy/testing/gpg.key' \
    | gpg --dearmor -o /usr/share/keyrings/caddy-testing-archive-keyring.gpg \
    && curl -1sLf 'https://dl.cloudsmith.io/public/caddy/testing/debian.deb.txt' \
    | tee /etc/apt/sources.list.d/caddy-testing.list \
    && apt-get update \
    && apt-get install -y caddy

COPY matrixon-example.toml matrixon.toml
COPY complement/caddy.json caddy.json

ENV SERVER_NAME=localhost
ENV matrixon_CONFIG=/workdir/matrixon.toml

RUN sed -i "s/port = 6167/port = 8008/g" matrixon.toml
RUN echo "log = \"warn,_=off,sled=off\"" >> matrixon.toml
RUN sed -i "s/address = \"127.0.0.1\"/address = \"0.0.0.0\"/g" matrixon.toml

EXPOSE 8008 8448

CMD uname -a && \
    sed -i "s/#server_name = \"your.server.name\"/server_name = \"${SERVER_NAME}\"/g" matrixon.toml && \
    sed -i "s/your.server.name/${SERVER_NAME}/g" caddy.json && \
    caddy start --config caddy.json > /dev/null && \
    /workdir/matrixon
