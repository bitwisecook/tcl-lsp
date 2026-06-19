//! Flow pairing: directional flows → Connections → client/server Sessions.
//! Faithful port of `dialects/f5/bigip/flow/sessions.py`.

use std::collections::HashMap;

use indexmap::IndexMap;

use super::model::{Connection, Flow, FlowKey, Session};

/// Pair opposite-direction flows into bidirectional [`Connection`]s.
///
/// The SYN-bearer is preferred as the client side. Each flow appears in exactly
/// one connection; orphan flows (no reverse seen) get `server = None`.
#[must_use]
pub fn pair_connections(flows: &IndexMap<FlowKey, Flow>) -> Vec<Connection> {
    let mut remaining: HashMap<FlowKey, Flow> =
        flows.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

    // Stable sort over the insertion-ordered keys by (syn-rank, -packets).
    let syn_rank = |f: &Flow| -> u8 {
        if f.tcp_syn {
            0
        } else if f.tcp_synack {
            1
        } else {
            2
        }
    };
    let mut keys_in_order: Vec<FlowKey> = flows.keys().cloned().collect();
    keys_in_order.sort_by(|a, b| {
        let fa = &flows[a];
        let fb = &flows[b];
        syn_rank(fa)
            .cmp(&syn_rank(fb))
            .then_with(|| fb.packets.cmp(&fa.packets))
    });

    let mut out: Vec<Connection> = Vec::new();
    for key in keys_in_order {
        let Some(mut flow) = remaining.remove(&key) else {
            continue;
        };
        let rev: FlowKey = (
            flow.dst_ip.clone(),
            flow.dst_port,
            flow.src_ip.clone(),
            flow.src_port,
            flow.proto,
        );
        let mut peer = remaining.remove(&rev);
        if let Some(p) = &peer
            && !flow.tcp_syn
            && p.tcp_syn
        {
            std::mem::swap(&mut flow, peer.as_mut().unwrap());
        }
        out.push(Connection {
            client: flow,
            server: peer,
        });
    }
    out
}

/// Pair flows into Connections, then pair Connections into Sessions.
///
/// On a `tcpdump -i <vlan>:np` capture every TMM-mediated packet appears twice,
/// each carrying the proxied peer-side 5-tuple in its F5 ethernet trailer. The
/// front-side client flow's `peer_*` fields point at the back-side connection.
#[must_use]
pub fn pair_sessions(flows: &IndexMap<FlowKey, Flow>) -> Vec<Session> {
    let conns = pair_connections(flows);

    // Index every connection by the 5-tuple of its client flow.
    let mut by_client_key: HashMap<FlowKey, usize> = HashMap::new();
    for (i, c) in conns.iter().enumerate() {
        by_client_key.insert(c.client.key(), i);
    }

    let mut used: Vec<bool> = vec![false; conns.len()];
    let mut sessions: Vec<Session> = Vec::new();
    for i in 0..conns.len() {
        if used[i] {
            continue;
        }
        let f = &conns[i].client;
        if !f.peer_remote_ip.is_empty()
            && f.peer_remote_port != 0
            && !f.peer_local_ip.is_empty()
            && f.peer_local_port != 0
        {
            let back_key: FlowKey = (
                f.peer_local_ip.clone(),
                f.peer_local_port,
                f.peer_remote_ip.clone(),
                f.peer_remote_port,
                f.proto,
            );
            if let Some(&j) = by_client_key.get(&back_key)
                && j != i
            {
                used[i] = true;
                used[j] = true;
                sessions.push(Session {
                    front: conns[i].clone(),
                    back: Some(conns[j].clone()),
                });
                continue;
            }
        }
        used[i] = true;
        sessions.push(Session {
            front: conns[i].clone(),
            back: None,
        });
    }
    sessions
}
