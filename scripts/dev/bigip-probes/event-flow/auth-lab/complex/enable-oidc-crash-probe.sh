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

name=/Common/__tcl_lsp_evtflow_efo_complex

tmsh modify apm policy agent aaa-oauth "${name}_oidc_auth_ag" \
    openid-connect enabled \
    openid-flow-type code \
    openid-userinfo-request "${name}_userinfo_request"
tmsh modify apm profile access "${name}_access" generation-action increment
tmsh save sys config

echo "OIDC mode enabled. On BIG-IP 21.1.0.1 build 0.0.26 this fixture"
echo "reproduced an apmd core in OAuth::updateOpenIDStats after token receipt."
echo "Restore the retained safe OAuth mode with:"
echo "tmsh modify apm policy agent aaa-oauth ${name}_oidc_auth_ag openid-connect disabled openid-userinfo-request none"
