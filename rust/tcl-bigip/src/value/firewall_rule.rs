//! Typed BIG-IP firewall + NAT rule values.

/// One side (source or destination) of a firewall rule.
///
/// Captures the lists of port-list refs, port literals + ranges, and
/// address-list refs. Each list keeps the original ordering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FirewallEndpoint {
    /// Port-list references.
    pub port_lists: Vec<String>,
    /// Port literals and ranges.
    pub ports: Vec<String>,
    /// Address-list references.
    pub address_lists: Vec<String>,
    /// Inline address literals.
    pub addresses: Vec<String>,
}

impl FirewallEndpoint {
    /// Parse an endpoint body, extracting the `port-lists` / `ports` /
    /// `address-lists` / `addresses` braced sub-lists.
    #[must_use]
    pub fn from_raw(body: &str) -> Self {
        let mut port_lists: Vec<String> = Vec::new();
        let mut ports: Vec<String> = Vec::new();
        let mut address_lists: Vec<String> = Vec::new();
        let mut addresses: Vec<String> = Vec::new();
        let tokens: Vec<&str> = body.split_whitespace().collect();
        let mut idx = 0;
        while idx < tokens.len() {
            let tok = tokens[idx];
            if matches!(tok, "port-lists" | "ports" | "address-lists" | "addresses")
                && idx + 1 < tokens.len()
                && tokens[idx + 1] == "{"
            {
                let target = match tok {
                    "port-lists" => &mut port_lists,
                    "ports" => &mut ports,
                    "address-lists" => &mut address_lists,
                    _ => &mut addresses,
                };
                idx += 2;
                while idx < tokens.len() && tokens[idx] != "}" {
                    target.push(tokens[idx].to_owned());
                    idx += 1;
                }
                if idx < tokens.len() {
                    idx += 1;
                }
                continue;
            }
            idx += 1;
        }
        Self {
            port_lists,
            ports,
            address_lists,
            addresses,
        }
    }

    /// Yield `(kind, full-path)` pairs for every list reference.
    #[must_use]
    pub fn references(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for path in &self.port_lists {
            out.push(("security firewall port-list".to_owned(), path.clone()));
        }
        for path in &self.address_lists {
            out.push(("security firewall address-list".to_owned(), path.clone()));
        }
        out
    }
}

/// One rule inside a firewall rule-list / policy.
///
/// Exposes the classical 5-tuple as a structured value. `raw` preserves
/// the original body for round trips and for less-common variants this
/// value doesn't surface explicitly. [`NatRule`] is an alias for this
/// type (NAT rules share the firewall rule shape).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FirewallRule {
    /// Rule name (the key in the parent `rules { ... }` block).
    pub name: String,
    /// Action verb (`accept` / `drop` / `reject` / ...).
    pub action: String,
    /// TMSH-spelt protocol (`tcp` / `udp` / `icmp` / `any`).
    pub ip_protocol: String,
    /// Whether the rule logs matches.
    pub log: bool,
    /// Source endpoint.
    pub source: FirewallEndpoint,
    /// Destination endpoint.
    pub destination: FirewallEndpoint,
    /// Rule-list reference (when the rule is a list-reference).
    pub rule_list: String,
    /// Original brace-body text (stripped).
    pub raw: String,
}

impl FirewallRule {
    /// Build a rule from its keyed-block body.
    #[must_use]
    pub fn from_raw(name: &str, body: &str) -> Self {
        let mut action = String::new();
        let mut ip_protocol = String::new();
        let mut log = false;
        let mut rule_list = String::new();
        let mut source = FirewallEndpoint::default();
        let mut destination = FirewallEndpoint::default();

        let (tokens, sub_bodies) = split_sub_blocks(body);
        let mut idx = 0;
        while idx < tokens.len() {
            let tok = tokens[idx].as_str();
            if tok == "action" && idx + 1 < tokens.len() {
                tokens[idx + 1].clone_into(&mut action);
                idx += 2;
                continue;
            }
            if tok == "ip-protocol" && idx + 1 < tokens.len() {
                tokens[idx + 1].clone_into(&mut ip_protocol);
                idx += 2;
                continue;
            }
            if tok == "log" {
                log = true;
                idx += 1;
                continue;
            }
            if tok == "rule-list" && idx + 1 < tokens.len() {
                tokens[idx + 1].clone_into(&mut rule_list);
                idx += 2;
                continue;
            }
            idx += 1;
        }
        if let Some(b) = sub_bodies.iter().find(|(k, _)| k == "source") {
            source = FirewallEndpoint::from_raw(&b.1);
        }
        if let Some(b) = sub_bodies.iter().find(|(k, _)| k == "destination") {
            destination = FirewallEndpoint::from_raw(&b.1);
        }
        Self {
            name: name.to_owned(),
            action,
            ip_protocol,
            log,
            source,
            destination,
            rule_list,
            raw: body.trim().to_owned(),
        }
    }

    /// Yield every outbound reference the rule carries.
    #[must_use]
    pub fn references(&self) -> Vec<(String, String)> {
        let mut out = self.source.references();
        out.extend(self.destination.references());
        if !self.rule_list.is_empty() {
            out.push((
                "security firewall rule-list".to_owned(),
                self.rule_list.clone(),
            ));
        }
        out
    }
}

/// NAT rules share the firewall rule shape, modelled as a thin alias.
pub type NatRule = FirewallRule;

/// Tokenise `body` into top-level tokens + an ordered `(name, sub-body)`
/// list of `source` / `destination` sub-block contents, handling brace
/// nesting.
fn split_sub_blocks(body: &str) -> (Vec<String>, Vec<(String, String)>) {
    let mut sub_bodies: Vec<(String, String)> = Vec::new();
    let mut tokens: Vec<String> = Vec::new();
    let raw_tokens: Vec<&str> = body.split_whitespace().collect();
    let mut idx = 0;
    while idx < raw_tokens.len() {
        let tok = raw_tokens[idx];
        if matches!(tok, "source" | "destination")
            && idx + 1 < raw_tokens.len()
            && raw_tokens[idx + 1] == "{"
        {
            let mut depth = 1i32;
            idx += 2;
            let mut sub_parts: Vec<&str> = Vec::new();
            while idx < raw_tokens.len() && depth > 0 {
                let t = raw_tokens[idx];
                if t == "{" {
                    depth += 1;
                } else if t == "}" {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                sub_parts.push(t);
                idx += 1;
            }
            if idx < raw_tokens.len() {
                idx += 1; // skip closing brace
            }
            // Last assignment for a repeated key wins.
            let joined = sub_parts.join(" ");
            if let Some(slot) = sub_bodies.iter_mut().find(|(k, _)| k == tok) {
                slot.1 = joined;
            } else {
                sub_bodies.push((tok.to_owned(), joined));
            }
            continue;
        }
        tokens.push(tok.to_owned());
        idx += 1;
    }
    (tokens, sub_bodies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rule() {
        let body = "action accept ip-protocol tcp \
            destination { port-lists { /Common/_sys_self_allow_tcp_defaults } ports { 443 } address-lists { /Common/web_servers } } \
            source { address-lists { /Common/trusted_clients } ports { 1024-65535 } }";
        let r = FirewallRule::from_raw("allow_https", body);
        assert_eq!(r.action, "accept");
        assert_eq!(r.ip_protocol, "tcp");
        assert_eq!(r.destination.ports, vec!["443".to_owned()]);
        assert_eq!(
            r.destination.port_lists,
            vec!["/Common/_sys_self_allow_tcp_defaults".to_owned()]
        );
        assert_eq!(
            r.source.address_lists,
            vec!["/Common/trusted_clients".to_owned()]
        );
        let refs = r.references();
        assert!(refs.contains(&(
            "security firewall address-list".to_owned(),
            "/Common/trusted_clients".to_owned()
        )));
    }

    #[test]
    fn rule_list_reference() {
        let r = FirewallRule::from_raw("ref", "rule-list /Common/shared_rules");
        assert_eq!(r.rule_list, "/Common/shared_rules");
        assert_eq!(
            r.references(),
            vec![(
                "security firewall rule-list".to_owned(),
                "/Common/shared_rules".to_owned()
            )]
        );
    }

    #[test]
    fn log_flag() {
        let r = FirewallRule::from_raw("r", "action drop log");
        assert!(r.log);
        assert_eq!(r.action, "drop");
    }

    #[test]
    fn nat_rule_is_alias() {
        let _: NatRule = FirewallRule::from_raw("n", "action accept");
    }
}
