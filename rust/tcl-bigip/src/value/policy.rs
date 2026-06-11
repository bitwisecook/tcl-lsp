//! Typed BIG-IP policy condition / action values. Rust port of
//! `_policy.py`.

use std::fmt;

const OPERATORS: &[&str] = &[
    "equals",
    "contains",
    "starts-with",
    "ends-with",
    "matches",
    "less",
    "greater",
    "less-or-equal",
    "greater-or-equal",
    "not-equal",
];

const COND_EVENTS: &[&str] = &[
    "request",
    "response",
    "proxy-request",
    "proxy-response",
    "proxy-connect",
    "client-accepted",
    "server-connected",
    "ssl-client-hello",
    "ssl-client-server-hello-send",
    "ws-request",
    "ws-response",
];

const ACTION_TARGETS: &[&str] = &[
    "request",
    "response",
    "proxy-request",
    "proxy-response",
    "proxy-connect",
    "client-accepted",
    "server-connected",
    "ssl-client-hello",
    "ws-request",
    "ws-response",
];

const ACTION_VERBS: &[&str] = &[
    "forward",
    "redirect",
    "insert",
    "remove",
    "replace",
    "log",
    "persist",
    "reset",
    "set-variable",
    "tcl",
    "enable",
    "disable",
    "drop",
    "decompress",
    "compress",
    "cache",
    "classify",
    "expire",
    "rewrite",
    "shutdown",
];

/// One clause inside a policy rule's `conditions { ... }` block.
///
/// Models the documented condition vocabulary; unknown tokens are
/// preserved via `raw` for round-trip fidelity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LtmPolicyCondition {
    /// Integer clause key.
    pub index: i64,
    /// Protocol element matched against.
    pub operand: String,
    /// Sub-attribute of the operand.
    pub selector: String,
    /// Match operator (defaults to `equals`).
    pub operator: String,
    /// Ordered literals the operator compares against.
    pub values: Vec<String>,
    /// `not` flag.
    pub negate: bool,
    /// `case-insensitive` flag.
    pub case_insensitive: bool,
    /// `external` flag.
    pub external: bool,
    /// Policy event the condition runs against.
    pub event: String,
    /// Original brace-body text (stripped).
    pub raw: String,
}

impl Default for LtmPolicyCondition {
    fn default() -> Self {
        Self {
            index: 0,
            operand: String::new(),
            selector: String::new(),
            operator: "equals".to_owned(),
            values: Vec::new(),
            negate: false,
            case_insensitive: false,
            external: false,
            event: String::new(),
            raw: String::new(),
        }
    }
}

impl LtmPolicyCondition {
    /// Build a condition from its keyed-block body.
    #[must_use]
    pub fn from_raw(index: i64, body: &str) -> Self {
        let tokens: Vec<&str> = body.split_whitespace().collect();
        let mut operand = String::new();
        let mut selector = String::new();
        let mut operator = "equals".to_owned();
        let mut values: Vec<String> = Vec::new();
        let mut negate = false;
        let mut case_insensitive = false;
        let mut external = false;
        let mut event = String::new();
        let mut idx = 0;
        while idx < tokens.len() {
            let tok = tokens[idx];
            if tok == "not" {
                negate = true;
                idx += 1;
                continue;
            }
            if tok == "case-insensitive" {
                case_insensitive = true;
                idx += 1;
                continue;
            }
            if tok == "external" {
                external = true;
                idx += 1;
                continue;
            }
            if OPERATORS.contains(&tok) {
                tok.clone_into(&mut operator);
                idx += 1;
                continue;
            }
            if COND_EVENTS.contains(&tok) {
                tok.clone_into(&mut event);
                idx += 1;
                continue;
            }
            if tok == "values" && idx + 1 < tokens.len() && tokens[idx + 1] == "{" {
                idx += 2;
                while idx < tokens.len() && tokens[idx] != "}" {
                    values.push(tokens[idx].to_owned());
                    idx += 1;
                }
                if idx < tokens.len() {
                    idx += 1;
                }
                continue;
            }
            if operand.is_empty() {
                tok.clone_into(&mut operand);
                idx += 1;
                continue;
            }
            if selector.is_empty() {
                tok.clone_into(&mut selector);
                idx += 1;
                continue;
            }
            idx += 1;
        }
        Self {
            index,
            operand,
            selector,
            operator,
            values,
            negate,
            case_insensitive,
            external,
            event,
            raw: body.trim().to_owned(),
        }
    }
}

impl fmt::Display for LtmPolicyCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.raw.is_empty() {
            return write!(f, "{} {{ {} }}", self.index, self.raw);
        }
        let mut parts: Vec<String> = Vec::new();
        if !self.event.is_empty() {
            parts.push(self.event.clone());
        }
        if !self.operand.is_empty() {
            parts.push(self.operand.clone());
        }
        if !self.selector.is_empty() {
            parts.push(self.selector.clone());
        }
        if self.negate {
            parts.push("not".to_owned());
        }
        if self.case_insensitive {
            parts.push("case-insensitive".to_owned());
        }
        if self.external {
            parts.push("external".to_owned());
        }
        if !self.operator.is_empty() && self.operator != "equals" {
            parts.push(self.operator.clone());
        }
        if !self.values.is_empty() {
            parts.push(format!("values {{ {} }}", self.values.join(" ")));
        }
        let body = parts.join(" ");
        if body.is_empty() {
            write!(f, "{} {{ }}", self.index)
        } else {
            write!(f, "{} {{ {body} }}", self.index)
        }
    }
}

/// One clause inside a policy rule's `actions { ... }` block.
///
/// Models the common slots explicitly and keeps the raw body for less
/// common variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct LtmPolicyAction {
    /// Integer clause key.
    pub index: i64,
    /// Action target (`request` / `response` / ...).
    pub target: String,
    /// Action verb.
    pub verb: String,
    /// `select` flag.
    pub select: bool,
    /// Pool reference.
    pub pool: String,
    /// Redirect location.
    pub redirect_location: String,
    /// Status code.
    pub status: Option<i64>,
    /// Original brace-body text (stripped).
    pub raw: String,
}

impl LtmPolicyAction {
    /// Build an action from its keyed-block body.
    #[must_use]
    pub fn from_raw(index: i64, body: &str) -> Self {
        let tokens: Vec<&str> = body.split_whitespace().collect();
        let mut target = String::new();
        let mut verb = String::new();
        let mut select = false;
        let mut pool = String::new();
        let mut redirect_location = String::new();
        let mut status: Option<i64> = None;
        let mut idx = 0;
        while idx < tokens.len() {
            let tok = tokens[idx];
            if ACTION_TARGETS.contains(&tok) && target.is_empty() {
                tok.clone_into(&mut target);
                idx += 1;
                continue;
            }
            if ACTION_VERBS.contains(&tok) && verb.is_empty() {
                tok.clone_into(&mut verb);
                idx += 1;
                continue;
            }
            if tok == "select" {
                select = true;
                idx += 1;
                continue;
            }
            if tok == "pool" && idx + 1 < tokens.len() {
                tokens[idx + 1].clone_into(&mut pool);
                idx += 2;
                continue;
            }
            if tok == "location" && idx + 1 < tokens.len() {
                tokens[idx + 1].clone_into(&mut redirect_location);
                idx += 2;
                continue;
            }
            if tok == "status" && idx + 1 < tokens.len() {
                status = tokens[idx + 1].parse::<i64>().ok();
                idx += 2;
                continue;
            }
            idx += 1;
        }
        Self {
            index,
            target,
            verb,
            select,
            pool,
            redirect_location,
            status,
            raw: body.trim().to_owned(),
        }
    }

    /// Yield `(kind, full-path)` pairs for outbound references.
    #[must_use]
    pub fn references(&self) -> Vec<(String, String)> {
        if self.verb == "forward" && !self.pool.is_empty() {
            return vec![("ltm pool".to_owned(), self.pool.clone())];
        }
        Vec::new()
    }
}

impl fmt::Display for LtmPolicyAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.raw.is_empty() {
            return write!(f, "{} {{ {} }}", self.index, self.raw);
        }
        let mut parts: Vec<String> = Vec::new();
        if !self.target.is_empty() {
            parts.push(self.target.clone());
        }
        if !self.verb.is_empty() {
            parts.push(self.verb.clone());
        }
        if self.select {
            parts.push("select".to_owned());
        }
        if !self.pool.is_empty() {
            parts.push(format!("pool {}", self.pool));
        }
        if !self.redirect_location.is_empty() {
            parts.push(format!("location {}", self.redirect_location));
        }
        if let Some(status) = self.status {
            parts.push(format!("status {status}"));
        }
        let body = parts.join(" ");
        if body.is_empty() {
            write!(f, "{} {{ }}", self.index)
        } else {
            write!(f, "{} {{ {body} }}", self.index)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_from_raw() {
        let c = LtmPolicyCondition::from_raw(0, "http-host all-strings values { example.com }");
        assert_eq!(c.operand, "http-host");
        assert_eq!(c.selector, "all-strings");
        assert_eq!(c.values, vec!["example.com".to_owned()]);
        assert_eq!(
            c.to_string(),
            "0 { http-host all-strings values { example.com } }"
        );
    }

    #[test]
    fn condition_structured_render() {
        let c = LtmPolicyCondition {
            index: 1,
            operand: "http-uri".to_owned(),
            operator: "contains".to_owned(),
            values: vec!["a".to_owned(), "b".to_owned()],
            negate: true,
            ..Default::default()
        };
        assert_eq!(c.to_string(), "1 { http-uri not contains values { a b } }");
    }

    #[test]
    fn action_forward() {
        let a = LtmPolicyAction::from_raw(0, "forward select pool /Common/web_pool");
        assert_eq!(a.verb, "forward");
        assert!(a.select);
        assert_eq!(a.pool, "/Common/web_pool");
        assert_eq!(
            a.references(),
            vec![("ltm pool".to_owned(), "/Common/web_pool".to_owned())]
        );
        assert_eq!(a.to_string(), "0 { forward select pool /Common/web_pool }");
    }

    #[test]
    fn action_structured_render() {
        let a = LtmPolicyAction {
            index: 2,
            target: "request".to_owned(),
            verb: "redirect".to_owned(),
            redirect_location: "https://x".to_owned(),
            status: Some(302),
            ..Default::default()
        };
        assert_eq!(
            a.to_string(),
            "2 { request redirect location https://x status 302 }"
        );
    }
}
