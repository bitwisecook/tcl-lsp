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
vmid="${EVTFLOW_AD_VMID:-104}"
template="${EVTFLOW_AD_TEMPLATE:-local:vztmpl/debian-12-standard_12.12-1_amd64.tar.zst}"
storage="${EVTFLOW_AD_STORAGE:-nvme-zfs}"
bridge="${EVTFLOW_AD_BRIDGE:-vmbr0}"
hostname=ad.lab.bitwisecook.org
address=192.168.9.251/24
temporary_gateway=192.168.9.1

if pct status "$vmid" >/dev/null 2>&1; then
    existing_hostname="$(pct config "$vmid" | sed -n 's/^hostname: //p')"
    if [ "$existing_hostname" != "$hostname" ]; then
        echo "refusing to alter VMID $vmid owned by $existing_hostname" >&2
        exit 1
    fi
else
    pct create "$vmid" "$template" \
        --arch amd64 \
        --cores 2 \
        --features nesting=1,keyctl=1 \
        --hostname "$hostname" \
        --memory 2048 \
        --net0 "name=eth0,bridge=$bridge,firewall=1,gw=$temporary_gateway,ip=$address,type=veth" \
        --onboot 0 \
        --ostype debian \
        --rootfs "$storage:16" \
        --swap 512 \
        --unprivileged 0
fi

if [ "$(pct status "$vmid" | awk '{print $2}')" != running ]; then
    pct start "$vmid"
fi

pct exec "$vmid" -- env DEBIAN_FRONTEND=noninteractive apt-get update
pct exec "$vmid" -- env DEBIAN_FRONTEND=noninteractive apt-get install --yes \
    attr dnsutils krb5-user ldap-utils samba-ad-dc samba-ad-provision \
    samba-dsdb-modules winbind

pct push "$vmid" "$script_dir/samba/provision-ad.sh" \
    /usr/local/sbin/evtflow-samba-provision --perms 0755
pct exec "$vmid" -- env \
    EVTFLOW_AD_ADMIN_PASSWORD='Evtflow-Ad-Admin-2026!' \
    EVTFLOW_AD_USER_PASSWORD='Evtflow-Ad-User-2026!' \
    /usr/local/sbin/evtflow-samba-provision

pct exec "$vmid" -- systemctl mask smbd nmbd winbind
pct exec "$vmid" -- systemctl unmask samba-ad-dc
pct exec "$vmid" -- systemctl enable samba-ad-dc
pct exec "$vmid" -- systemctl restart samba-ad-dc

# Retained runtime is limited to the directly connected lab subnet. Package
# installation above is the only phase with a default gateway.
pct set "$vmid" \
    --nameserver 192.168.9.251 \
    --net0 "name=eth0,bridge=$bridge,firewall=1,ip=$address,type=veth" \
    --searchdomain lab.bitwisecook.org
pct reboot "$vmid"

for _ in $(seq 1 60); do
    if pct exec "$vmid" -- samba-tool user show evtflow-ad >/dev/null 2>&1; then
        break
    fi
    sleep 2
done

pct config "$vmid"
pct exec "$vmid" -- samba-tool domain level show
pct exec "$vmid" -- samba-tool group listmembers evtflow-employees
pct exec "$vmid" -- ip route
if pct exec "$vmid" -- ip route get 1.1.1.1 >/dev/null 2>&1; then
    echo "unexpected external route from retained AD LXC" >&2
    exit 1
fi
