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

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
network=evtflow-auth-macvlan
keycloak_image=quay.io/keycloak/keycloak:26.3.3
client_image=docker.io/curlimages/curl:8.16.0
pki_dir="$script_dir/keycloak/pki"

mkdir -p "$pki_dir"
if [ ! -s "$pki_dir/keycloak.crt" ] || [ ! -s "$pki_dir/keycloak.key" ]; then
    openssl req -new -newkey rsa:2048 -nodes -x509 -days 365 \
        -keyout "$pki_dir/keycloak.key" \
        -out "$pki_dir/keycloak.crt" \
        -subj '/CN=oidc.lab.bitwisecook.org/O=tcl-lsp event-flow/C=GB' \
        -addext 'subjectAltName=DNS:oidc.lab.bitwisecook.org,IP:192.168.9.252'
fi

if ! sudo podman network exists "$network"; then
    sudo podman network create \
        --driver macvlan \
        --internal \
        --subnet 192.168.9.0/24 \
        -o parent=eth0 \
        "$network"
fi

# Pull while the engine still has its ordinary build network. Runtime is
# attached only to the no-gateway lab macvlan. Samba AD is deployed by the
# peer Proxmox recipe because SYSVOL requires a real xattr-capable filesystem.
sudo podman pull "$keycloak_image"
sudo podman pull "$client_image"

sudo podman rm --force evtflow-keycloak evtflow-auth-client 2>/dev/null || true

sudo podman run --detach \
    --name evtflow-keycloak \
    --hostname oidc.lab.bitwisecook.org \
    --network "$network" \
    --ip 192.168.9.252 \
    --env KC_BOOTSTRAP_ADMIN_USERNAME=evtflow-admin \
    --env KC_BOOTSTRAP_ADMIN_PASSWORD='Evtflow-Oidc-Admin-2026!' \
    --volume "$script_dir/keycloak/realm-evtflow.json:/opt/keycloak/data/import/realm-evtflow.json:ro,Z" \
    --volume "$pki_dir/keycloak.crt:/opt/keycloak/conf/keycloak.crt:ro,Z" \
    --volume "$pki_dir/keycloak.key:/opt/keycloak/conf/keycloak.key:ro,Z" \
    "$keycloak_image" \
    start-dev \
    --http-enabled=false \
    --https-port=8443 \
    --https-certificate-file=/opt/keycloak/conf/keycloak.crt \
    --https-certificate-key-file=/opt/keycloak/conf/keycloak.key \
    --hostname=https://192.168.9.252:8443 \
    --hostname-strict=false \
    --import-realm

sudo podman run --detach \
    --name evtflow-auth-client \
    --hostname client.lab.bitwisecook.org \
    --network "$network" \
    --ip 192.168.9.253 \
    --add-host app.bitwisecook.org:192.168.9.24 \
    --volume "$script_dir/client/oidc-flow.sh:/usr/local/bin/oidc-flow.sh:ro,Z" \
    "$client_image" \
    sleep infinity

for _ in $(seq 1 90); do
    for container in evtflow-keycloak evtflow-auth-client; do
        if [ "$(sudo podman inspect --format '{{.State.Running}}' "$container")" != true ]; then
            sudo podman logs "$container" >&2
            echo "$container exited before becoming ready" >&2
            exit 1
        fi
    done
    if sudo podman exec evtflow-keycloak \
            bash -c 'exec 3<>/dev/tcp/127.0.0.1/8443'; then
        break
    fi
    sleep 2
done

sudo podman inspect evtflow-keycloak evtflow-auth-client \
    --format '{{.Name}} image={{.ImageName}} ip={{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}'

if sudo podman exec evtflow-keycloak \
    timeout 2 bash -c 'exec 3<>/dev/tcp/1.1.1.1/443' >/dev/null 2>&1; then
    echo "unexpected external route from isolated OIDC container" >&2
    exit 1
fi
if sudo podman exec evtflow-auth-client ip route | grep -q '^default '; then
    echo "unexpected default route in isolated OIDC client" >&2
    exit 1
fi
