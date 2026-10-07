#!/bin/bash
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

base_image=docker.io/bitnamilegacy/openldap:2.6.8
image=localhost/evtflow-openldap:2.6.8
network=evtflow-auth-macvlan
container=evtflow-openldap
ldap_address=192.168.9.250
script_dir=$(cd -- "$(dirname -- "$0")" && pwd)

container_runtime=(sudo podman)

"${container_runtime[@]}" pull "$base_image"
"${container_runtime[@]}" build --tag "$image" "$script_dir"
if ! "${container_runtime[@]}" network inspect "$network" >/dev/null 2>&1; then
    "${container_runtime[@]}" network create \
        --driver macvlan \
        --internal \
        --subnet 192.168.9.0/24 \
        --ip-range 192.168.9.248/29 \
        --opt parent=eth0 \
        "$network"
fi
if "${container_runtime[@]}" container exists "$container"; then
    "${container_runtime[@]}" rm --force "$container"
fi
"${container_runtime[@]}" run --detach \
    --name "$container" \
    --network "$network" \
    --ip "$ldap_address" \
    --env LDAP_ROOT=dc=bitwisecook,dc=org \
    --env LDAP_ADMIN_USERNAME=admin \
    --env LDAP_ADMIN_PASSWORD=evtflow-admin-2026 \
    --env LDAP_CUSTOM_LDIF_DIR=/ldifs \
    --env LDAP_ENABLE_TLS=no \
    "$image"

"${container_runtime[@]}" inspect "$container" --format '{{.ImageDigest}}'
"${container_runtime[@]}" inspect "$container" --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}'
