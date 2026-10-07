#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 OUTPUT_DIRECTORY" >&2
    exit 2
fi

output=$1
if [[ -e $output ]]; then
    echo "output already exists: $output" >&2
    exit 2
fi

umask 077
mkdir -p "$output/private" "$output/public"

openssl req -new -newkey rsa:2048 -nodes -x509 -days 30 \
    -keyout "$output/private/ca.key" \
    -out "$output/public/ca.crt" \
    -subj "/C=GB/O=tcl-lsp event-flow/CN=evtflow test CA" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" \
    >"$output/ca-generate.log" 2>&1

openssl req -new -newkey rsa:2048 -nodes \
    -keyout "$output/private/server.key" \
    -out "$output/server.csr" \
    -subj "/C=GB/O=tcl-lsp event-flow/CN=bigip.bitwisecook.org" \
    -addext "subjectAltName=DNS:bigip.bitwisecook.org,DNS:evtflow.bitwisecook.org,IP:192.168.9.24" \
    >"$output/server-generate.log" 2>&1
openssl x509 -req -days 30 -set_serial 4097 \
    -in "$output/server.csr" \
    -CA "$output/public/ca.crt" \
    -CAkey "$output/private/ca.key" \
    -copy_extensions copy \
    -out "$output/public/server.crt" \
    >>"$output/server-generate.log" 2>&1

openssl req -new -newkey rsa:2048 -nodes \
    -keyout "$output/private/client-trusted.key" \
    -out "$output/client-trusted.csr" \
    -subj "/C=GB/O=tcl-lsp event-flow/CN=trusted-client" \
    -addext "extendedKeyUsage=clientAuth" \
    >"$output/client-trusted-generate.log" 2>&1
openssl x509 -req -days 30 -set_serial 4098 \
    -in "$output/client-trusted.csr" \
    -CA "$output/public/ca.crt" \
    -CAkey "$output/private/ca.key" \
    -copy_extensions copy \
    -out "$output/public/client-trusted.crt" \
    >>"$output/client-trusted-generate.log" 2>&1

openssl req -new -newkey rsa:2048 -nodes -x509 -days 30 \
    -keyout "$output/private/rogue-ca.key" \
    -out "$output/public/rogue-ca.crt" \
    -subj "/C=GB/O=tcl-lsp event-flow/CN=evtflow rogue CA" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" \
    >"$output/rogue-ca-generate.log" 2>&1
openssl req -new -newkey rsa:2048 -nodes \
    -keyout "$output/private/client-untrusted.key" \
    -out "$output/client-untrusted.csr" \
    -subj "/C=GB/O=tcl-lsp event-flow/CN=untrusted-client" \
    -addext "extendedKeyUsage=clientAuth" \
    >"$output/client-untrusted-generate.log" 2>&1
openssl x509 -req -days 30 -set_serial 8193 \
    -in "$output/client-untrusted.csr" \
    -CA "$output/public/rogue-ca.crt" \
    -CAkey "$output/private/rogue-ca.key" \
    -copy_extensions copy \
    -out "$output/public/client-untrusted.crt" \
    >>"$output/client-untrusted-generate.log" 2>&1

openssl verify -CAfile "$output/public/ca.crt" \
    "$output/public/server.crt" "$output/public/client-trusted.crt" \
    >"$output/verify.log" 2>&1
if openssl verify -CAfile "$output/public/ca.crt" \
    "$output/public/client-untrusted.crt" >>"$output/verify.log" 2>&1; then
    echo "untrusted client unexpectedly verified" >&2
    exit 1
fi

for cert in "$output"/public/*.crt; do
    name=$(basename "$cert")
    openssl x509 -in "$cert" -noout -subject -issuer -serial -dates \
        -fingerprint -sha256 >"$output/public/$name.txt"
done

find "$output" -type f -not -name SHA256SUMS -print0 \
    | sort -z \
    | xargs -0 sha256sum \
    | sed "s#  $output/#  #" >"$output/SHA256SUMS"
chmod 0600 "$output/private"/*
chmod 0644 "$output/public"/* "$output"/*.csr "$output"/*.log "$output/SHA256SUMS"
