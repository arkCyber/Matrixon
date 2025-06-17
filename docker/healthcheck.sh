#!/bin/sh

# If the config file does not contain a default port and the matrixon_PORT env is not set, create
# try to get port from process list
if [ -z "${matrixon_PORT}" ]; then
  matrixon_PORT=$(ss -tlpn | grep matrixon | grep -m1 -o ':[0-9]*' | grep -m1 -o '[0-9]*')
fi

# If matrixon_ADDRESS is not set try to get the address from the process list
if [ -z "${matrixon_ADDRESS}" ]; then
  matrixon_ADDRESS=$(ss -tlpn | awk -F ' +|:' '/matrixon/ { print $4 }')
fi

# The actual health check.
# We try to first get a response on HTTP and when that fails on HTTPS and when that fails, we exit with code 1.
# TODO: Change this to a single wget call. Do we have a config value that we can check for that?
wget --no-verbose --tries=1 --spider "http://${matrixon_ADDRESS}:${matrixon_PORT}/_matrix/client/versions" || \
    wget --no-verbose --tries=1 --spider "https://${matrixon_ADDRESS}:${matrixon_PORT}/_matrix/client/versions" || \
    exit 1
