# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Flow pairing: directional flows -> Connections -> client/server Sessions."""

from __future__ import annotations

from ._model import Connection, Flow, Session


def pair_connections(flows: dict[tuple[str, int, str, int, int], Flow]) -> list[Connection]:
    """Pair opposite-direction flows into bidirectional :class:`Connection`s.

    The SYN-bearer is preferred as the client side.  Each flow appears
    in exactly one connection; orphan flows (no reverse seen) are
    emitted as a connection with ``server=None``.
    """
    remaining = dict(flows)
    out: list[Connection] = []
    keys_in_order = sorted(
        remaining.keys(),
        key=lambda k: (
            0 if remaining[k].tcp_syn else (1 if remaining[k].tcp_synack else 2),
            -remaining[k].packets,
        ),
    )
    for key in keys_in_order:
        if key not in remaining:
            continue
        flow = remaining.pop(key)
        rev = (flow.dst_ip, flow.dst_port, flow.src_ip, flow.src_port, flow.proto)
        peer = remaining.pop(rev, None)
        if peer is not None and not flow.tcp_syn and peer.tcp_syn:
            flow, peer = peer, flow
        out.append(Connection(client=flow, server=peer))
    return out


def pair_sessions(
    flows: dict[tuple[str, int, str, int, int], Flow],
) -> list[Session]:
    """Pair flows into Connections, then pair Connections into Sessions.

    On a `tcpdump -i <vlan>:np` capture every TMM-mediated packet
    appears twice — once on the front (client-facing) side and once on
    the back (pool-member-facing) side.  The F5 ethernet trailer's
    HIGH TLV carries the proxied peer-side 5-tuple, so we can match a
    front-side Connection ``(client_ip:cport <-> vip:vport)`` with the
    back-side Connection it generated ``(snat_ip:sport <-> member_ip:mport)``
    by looking at the front-side client flow's
    ``(peer_remote_ip, peer_remote_port, peer_local_ip, peer_local_port)``
    values.

    Sessions emit only-front when no peer info is present (single-side
    capture) and only-back if a back-side Connection exists with no
    matching front (rare; usually means the front got dropped before
    the trailer was emitted).
    """
    conns = pair_connections(flows)

    # Index every connection by the 5-tuple of its client flow so we can
    # look up the back-side via the peer info on the front-side client flow.
    by_client_key: dict[tuple[str, int, str, int, int], Connection] = {}
    for c in conns:
        by_client_key[c.client.key] = c

    used: set[int] = set()
    sessions: list[Session] = []
    for c in conns:
        if id(c) in used:
            continue
        # Try treating *c* as the front side; look up the back-side
        # connection whose client 5-tuple matches the front client's
        # peer tuple.
        f = c.client
        if f.peer_remote_ip and f.peer_remote_port and f.peer_local_ip and f.peer_local_port:
            # Front-side client → VIP, peer = local→remote on the back side
            # (TMM as the proxied client, member as the proxied server).
            back_key = (
                f.peer_local_ip,
                f.peer_local_port,
                f.peer_remote_ip,
                f.peer_remote_port,
                f.proto,
            )
            back = by_client_key.get(back_key)
            if back is not None and id(back) != id(c):
                used.add(id(c))
                used.add(id(back))
                sessions.append(Session(front=c, back=back))
                continue
        used.add(id(c))
        sessions.append(Session(front=c, back=None))
    return sessions


# ---------------------------------------------------------------------------
# Optional tshark enrichment.
# ---------------------------------------------------------------------------
