//! Flow pairing: directional flows → Connections → client/server Sessions.

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

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::super::model::{Flow, FlowKey};
    use super::{pair_connections, pair_sessions};

    fn flow(src: &str, sp: u16, dst: &str, dp: u16) -> Flow {
        Flow::new(src.into(), sp, dst.into(), dp, 6)
    }

    #[test]
    fn pair_connections_pairs_opposite_flows_with_syn_bearer_as_client() {
        let mut client = flow("10.0.0.1", 1000, "10.0.0.2", 80);
        client.tcp_syn = true;
        client.packets = 5;
        let mut server = flow("10.0.0.2", 80, "10.0.0.1", 1000);
        server.tcp_synack = true;
        server.packets = 4;

        let mut flows: IndexMap<FlowKey, Flow> = IndexMap::new();
        // Insert the server side first to show the SYN-bearer still wins.
        flows.insert(server.key(), server.clone());
        flows.insert(client.key(), client.clone());

        let conns = pair_connections(&flows);
        assert_eq!(conns.len(), 1);
        assert_eq!(
            conns[0].client.src_port, 1000,
            "the SYN-bearer is the client"
        );
        assert_eq!(
            conns[0].server.as_ref().map(|s| s.src_port),
            Some(80),
            "the reverse flow becomes the server"
        );
    }

    #[test]
    fn pair_connections_leaves_orphans_without_a_server() {
        let mut only = flow("10.0.0.1", 1000, "10.0.0.2", 80);
        only.tcp_syn = true;
        let mut flows: IndexMap<FlowKey, Flow> = IndexMap::new();
        flows.insert(only.key(), only.clone());
        let conns = pair_connections(&flows);
        assert_eq!(conns.len(), 1);
        assert!(
            conns[0].server.is_none(),
            "no reverse flow → server is None"
        );
    }

    #[test]
    fn pair_sessions_links_front_and_back_via_peer_trailer() {
        // A front-side client flow whose F5 trailer peer-tuple points at the
        // back-side connection's client 5-tuple.
        let mut front = flow("10.0.0.1", 1000, "10.0.0.2", 80);
        front.tcp_syn = true;
        front.peer_local_ip = "172.16.0.1".into();
        front.peer_local_port = 5000;
        front.peer_remote_ip = "192.168.1.1".into();
        front.peer_remote_port = 443;
        // The back connection's client flow, keyed exactly by that peer-tuple.
        let mut back = flow("172.16.0.1", 5000, "192.168.1.1", 443);
        back.tcp_syn = true;

        let mut flows: IndexMap<FlowKey, Flow> = IndexMap::new();
        flows.insert(front.key(), front.clone());
        flows.insert(back.key(), back.clone());

        let sessions = pair_sessions(&flows);
        assert_eq!(sessions.len(), 1, "front and back fold into one session");
        let s = &sessions[0];
        assert_eq!(s.front.client.dst_port, 80);
        let back_conn = s.back.as_ref().expect("back side is linked");
        assert_eq!(back_conn.client.src_ip, "172.16.0.1");
    }

    #[test]
    fn pair_sessions_without_peer_info_has_no_back() {
        let mut lone = flow("10.0.0.1", 1000, "10.0.0.2", 80);
        lone.tcp_syn = true;
        let mut flows: IndexMap<FlowKey, Flow> = IndexMap::new();
        flows.insert(lone.key(), lone.clone());
        let sessions = pair_sessions(&flows);
        assert_eq!(sessions.len(), 1);
        assert!(sessions[0].back.is_none());
    }
}
