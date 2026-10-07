#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -eu

realm=LAB.BITWISECOOK.ORG
domain=EVTFLOW
host=ad
address=192.168.9.251
state=/var/lib/samba/private/sam.ldb

: "${EVTFLOW_AD_ADMIN_PASSWORD:?missing AD administrator password}"
: "${EVTFLOW_AD_USER_PASSWORD:?missing AD test-user password}"

if [ ! -f "$state" ]; then
    rm -f /etc/samba/smb.conf
    samba-tool domain provision \
        --server-role=dc \
        --use-rfc2307 \
        --dns-backend=SAMBA_INTERNAL \
        --realm="$realm" \
        --domain="$domain" \
        --host-name="$host" \
        --host-ip="$address" \
        --adminpass="$EVTFLOW_AD_ADMIN_PASSWORD"

    samba-tool user create evtflow-ad "$EVTFLOW_AD_USER_PASSWORD" \
        --given-name=Event \
        --surname=Flow \
        --mail-address=evtflow-ad@bitwisecook.org
    samba-tool user setexpiry evtflow-ad --noexpiry
    samba-tool group add evtflow-employees
    samba-tool group addmembers evtflow-employees evtflow-ad
fi

cat >/etc/krb5.conf <<'EOF'
[libdefaults]
    default_realm = LAB.BITWISECOOK.ORG
    dns_lookup_realm = false
    dns_lookup_kdc = false

[realms]
    LAB.BITWISECOOK.ORG = {
        kdc = ad.lab.bitwisecook.org
        admin_server = ad.lab.bitwisecook.org
    }
EOF
