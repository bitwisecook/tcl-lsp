// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Static nginx TLS configuration adapter.
//!
//! The adapter parses only the supplied text. It never follows `include`
//! directives, expands nginx variables, opens certificate paths, or performs
//! network access.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config_adapter::{ConfigError, ConfigEvidence, ConfigNotice};
use crate::model::{Endpoint, HstsPolicy, ProtocolVersion, TlsValue};

/// TLS endpoints projected from one nginx configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NginxImport {
    /// One endpoint per literal `server` block containing TLS-facing state.
    pub endpoints: Vec<Endpoint>,
    /// Source directives and their effective/overridden state.
    pub evidence: Vec<ConfigEvidence>,
    /// Non-fatal constructs that were retained but not interpreted.
    pub notices: Vec<ConfigNotice>,
    /// Unsupported main-context constructs, retained structurally.
    pub extensions: BTreeMap<String, Vec<TlsValue>>,
    /// Exact input, including comments and unsupported syntax.
    pub raw: String,
}

#[derive(Debug, Clone)]
struct Word {
    text: String,
    dynamic: bool,
}

#[derive(Debug, Clone)]
enum TokenKind {
    Word(Word),
    Open,
    Close,
    End,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    line: u32,
}

#[derive(Debug, Clone)]
struct Node {
    name: String,
    args: Vec<Word>,
    children: Option<Vec<Node>>,
    line: u32,
}

impl Node {
    fn values(&self) -> Vec<String> {
        self.args.iter().map(|word| word.text.clone()).collect()
    }
}

#[derive(Debug, Clone)]
struct Context<'a> {
    section: String,
    nodes: &'a [Node],
}

#[derive(Debug, Default)]
struct Projection {
    endpoints: Vec<Endpoint>,
    evidence: Vec<ConfigEvidence>,
    notices: Vec<ConfigNotice>,
}

#[derive(Debug)]
struct Selection<'a> {
    all: Vec<(&'a Context<'a>, &'a Node)>,
    effective: Vec<&'a Node>,
    override_line: Option<u32>,
}

/// Parse nginx configuration text without evaluating it or following files.
pub fn import_nginx(source: &str) -> Result<NginxImport, ConfigError> {
    let tokens = tokenise(source)?;
    let nodes = parse_nodes(&tokens, &mut 0, false)?;
    let mut projection = Projection::default();
    collect_notices(&nodes, "main", &mut projection.notices);

    let root = Context {
        section: "main".to_owned(),
        nodes: &nodes,
    };
    collect_servers(&nodes, std::slice::from_ref(&root), &mut projection);
    if projection.endpoints.is_empty() && contains_tls_state(&nodes) {
        project_endpoint(&[root], &mut projection);
    }

    Ok(NginxImport {
        endpoints: projection.endpoints,
        evidence: projection.evidence,
        notices: projection.notices,
        extensions: main_extensions(&nodes),
        raw: source.to_owned(),
    })
}

fn tokenise(source: &str) -> Result<Vec<Token>, ConfigError> {
    let mut tokens = Vec::new();
    let mut text = String::new();
    let mut word_line = 1;
    let mut line = 1_u32;
    let mut dynamic = false;
    let mut quote = None;
    let mut escaped = false;
    let mut comment = false;

    for character in source.chars() {
        if comment {
            if character == '\n' {
                comment = false;
                line = line.saturating_add(1);
            }
            continue;
        }
        if escaped {
            escaped = false;
            if character == '\n' {
                line = line.saturating_add(1);
            } else {
                text.push(character);
            }
            continue;
        }
        if let Some(delimiter) = quote {
            match character {
                '\\' => escaped = true,
                value if value == delimiter => quote = None,
                '\n' => {
                    text.push(character);
                    line = line.saturating_add(1);
                }
                '$' => {
                    dynamic = true;
                    text.push(character);
                }
                _ => text.push(character),
            }
            continue;
        }

        match character {
            '#' => {
                push_word(&mut tokens, &mut text, &mut dynamic, word_line);
                comment = true;
            }
            '\\' => {
                if text.is_empty() {
                    word_line = line;
                }
                escaped = true;
            }
            '\'' | '"' => {
                if text.is_empty() {
                    word_line = line;
                }
                quote = Some(character);
            }
            '{' | '}' | ';' => {
                push_word(&mut tokens, &mut text, &mut dynamic, word_line);
                let kind = match character {
                    '{' => TokenKind::Open,
                    '}' => TokenKind::Close,
                    _ => TokenKind::End,
                };
                tokens.push(Token { kind, line });
            }
            '$' => {
                if text.is_empty() {
                    word_line = line;
                }
                dynamic = true;
                text.push(character);
            }
            value if value.is_whitespace() => {
                push_word(&mut tokens, &mut text, &mut dynamic, word_line);
                if character == '\n' {
                    line = line.saturating_add(1);
                }
            }
            _ => {
                if text.is_empty() {
                    word_line = line;
                }
                text.push(character);
            }
        }
    }

    finish_tokens(tokens, text, dynamic, line, word_line, quote, escaped)
}

fn finish_tokens(
    mut tokens: Vec<Token>,
    mut text: String,
    mut dynamic: bool,
    line: u32,
    word_line: u32,
    quote: Option<char>,
    escaped: bool,
) -> Result<Vec<Token>, ConfigError> {
    if escaped {
        return Err(ConfigError {
            line,
            message: "trailing nginx escape".to_owned(),
        });
    }
    if quote.is_some() {
        return Err(ConfigError {
            line: word_line,
            message: "unterminated nginx quoted value".to_owned(),
        });
    }
    push_word(&mut tokens, &mut text, &mut dynamic, word_line);
    Ok(tokens)
}

fn push_word(tokens: &mut Vec<Token>, text: &mut String, dynamic: &mut bool, line: u32) {
    if !text.is_empty() {
        tokens.push(Token {
            kind: TokenKind::Word(Word {
                text: std::mem::take(text),
                dynamic: *dynamic,
            }),
            line,
        });
        *dynamic = false;
    }
}

fn parse_nodes(
    tokens: &[Token],
    index: &mut usize,
    nested: bool,
) -> Result<Vec<Node>, ConfigError> {
    let mut nodes = Vec::new();
    let mut words = Vec::new();
    let mut statement_line = 1;

    while let Some(token) = tokens.get(*index) {
        *index += 1;
        match &token.kind {
            TokenKind::Word(word) => {
                if words.is_empty() {
                    statement_line = token.line;
                }
                words.push(word.clone());
            }
            TokenKind::End => {
                nodes.push(make_node(&mut words, statement_line, None, token.line)?);
            }
            TokenKind::Open => {
                let children = parse_nodes(tokens, index, true)?;
                nodes.push(make_node(
                    &mut words,
                    statement_line,
                    Some(children),
                    token.line,
                )?);
            }
            TokenKind::Close => {
                if !words.is_empty() {
                    return Err(ConfigError {
                        line: token.line,
                        message: "nginx directive is missing `;` or `{`".to_owned(),
                    });
                }
                if nested {
                    return Ok(nodes);
                }
                return Err(ConfigError {
                    line: token.line,
                    message: "unexpected nginx `}`".to_owned(),
                });
            }
        }
    }

    if !words.is_empty() {
        return Err(ConfigError {
            line: statement_line,
            message: "nginx directive is missing `;` or `{`".to_owned(),
        });
    }
    if nested {
        return Err(ConfigError {
            line: tokens.last().map_or(1, |token| token.line),
            message: "unterminated nginx block".to_owned(),
        });
    }
    Ok(nodes)
}

fn make_node(
    words: &mut Vec<Word>,
    line: u32,
    children: Option<Vec<Node>>,
    delimiter_line: u32,
) -> Result<Node, ConfigError> {
    if words.is_empty() {
        return Err(ConfigError {
            line: delimiter_line,
            message: "empty nginx directive".to_owned(),
        });
    }
    let name = words.remove(0).text;
    Ok(Node {
        name,
        args: std::mem::take(words),
        children,
        line,
    })
}

fn collect_servers<'a>(nodes: &'a [Node], contexts: &[Context<'a>], projection: &mut Projection) {
    for node in nodes {
        let Some(children) = node.children.as_deref() else {
            continue;
        };
        let section = format!(
            "{}/{}@{}",
            contexts.last().unwrap().section,
            node.name,
            node.line
        );
        let context = Context {
            section,
            nodes: children,
        };
        let mut nested_contexts = contexts.to_vec();
        nested_contexts.push(context);
        if node.name.eq_ignore_ascii_case("server") {
            if server_uses_tls(children) {
                project_endpoint(&nested_contexts, projection);
            }
        } else {
            collect_servers(children, &nested_contexts, projection);
        }
    }
}

fn project_endpoint(contexts: &[Context<'_>], projection: &mut Projection) {
    let endpoint_section = contexts.last().unwrap().section.clone();
    let evidence_start = projection.evidence.len();
    let protocol_nodes = record_selection(
        contexts,
        "ssl_protocols",
        false,
        None,
        &mut projection.evidence,
    );
    let cipher_nodes = record_selection(
        contexts,
        "ssl_ciphers",
        false,
        None,
        &mut projection.evidence,
    );
    let cipher_suite_nodes = record_selection(
        contexts,
        "ssl_ciphersuites",
        false,
        None,
        &mut projection.evidence,
    );
    let certificate_nodes = record_selection(
        contexts,
        "ssl_certificate",
        true,
        None,
        &mut projection.evidence,
    );
    let key_nodes = record_selection(
        contexts,
        "ssl_certificate_key",
        true,
        None,
        &mut projection.evidence,
    );
    let server_name_nodes = record_selection(
        contexts,
        "server_name",
        false,
        None,
        &mut projection.evidence,
    );
    let listen_nodes = record_selection(contexts, "listen", true, None, &mut projection.evidence);
    let hsts_nodes = record_selection(
        contexts,
        "add_header",
        true,
        Some("add_header"),
        &mut projection.evidence,
    )
    .into_iter()
    .filter(|node| is_hsts(node))
    .collect::<Vec<_>>();

    let hostname = static_hostname(&server_name_nodes, &endpoint_section, projection);
    let name = endpoint_name(hostname.as_deref(), &listen_nodes, contexts.last().unwrap());
    for item in &mut projection.evidence[evidence_start..] {
        item.target = Some(name.clone());
    }
    let mut endpoint = Endpoint {
        name,
        hostname,
        ..Endpoint::default()
    };
    endpoint.protocols = project_protocols(&protocol_nodes, &endpoint_section, projection);
    endpoint.ciphers = project_colon_values(
        cipher_nodes
            .iter()
            .chain(cipher_suite_nodes.iter())
            .copied(),
        &endpoint_section,
        projection,
    );
    endpoint.certificate_chain = project_paths(
        &certificate_nodes,
        "ssl_certificate",
        &endpoint_section,
        projection,
    );
    endpoint.hsts = hsts_nodes
        .last()
        .map(|node| project_hsts(node, &endpoint_section, projection));
    add_endpoint_extensions(
        &mut endpoint,
        contexts.last().unwrap().nodes,
        &listen_nodes,
        &server_name_nodes,
        &key_nodes,
    );
    projection.endpoints.push(endpoint);
}

fn selection<'a>(
    contexts: &'a [Context<'a>],
    directive: &str,
    multiple: bool,
    blocker: Option<&str>,
) -> Selection<'a> {
    let mut all = Vec::new();
    let mut selected_context = None;
    for (context_index, context) in contexts.iter().enumerate() {
        let blocks_inheritance = context.nodes.iter().any(|node| {
            node.children.is_none() && node.name.eq_ignore_ascii_case(blocker.unwrap_or(directive))
        });
        for node in context
            .nodes
            .iter()
            .filter(|node| node.children.is_none() && node.name.eq_ignore_ascii_case(directive))
        {
            all.push((context, node));
        }
        if blocks_inheritance {
            selected_context = Some(context_index);
        }
    }

    let candidates = selected_context.map_or_else(Vec::new, |context_index| {
        contexts[context_index]
            .nodes
            .iter()
            .filter(|node| node.children.is_none() && node.name.eq_ignore_ascii_case(directive))
            .collect::<Vec<_>>()
    });
    let effective = if multiple {
        candidates
    } else {
        candidates.last().copied().into_iter().collect()
    };
    let override_line = selected_context.and_then(|context_index| {
        contexts[context_index]
            .nodes
            .iter()
            .find(|node| {
                node.children.is_none()
                    && node.name.eq_ignore_ascii_case(blocker.unwrap_or(directive))
            })
            .map(|node| node.line)
    });
    Selection {
        all,
        effective,
        override_line,
    }
}

fn record_selection<'a>(
    contexts: &'a [Context<'a>],
    directive: &str,
    multiple: bool,
    blocker: Option<&str>,
    evidence: &mut Vec<ConfigEvidence>,
) -> Vec<&'a Node> {
    let selected = selection(contexts, directive, multiple, blocker);
    for (context, node) in selected.all {
        let effective = selected
            .effective
            .iter()
            .any(|candidate| std::ptr::eq(*candidate, node));
        evidence.push(ConfigEvidence {
            line: node.line,
            section: context.section.clone(),
            target: None,
            directive: node.name.clone(),
            values: node.values(),
            raw_value: node.values().join(" "),
            effective,
            overridden_by_line: (!effective).then_some(selected.override_line).flatten(),
        });
    }
    selected.effective
}

fn static_hostname(
    server_names: &[&Node],
    section: &str,
    projection: &mut Projection,
) -> Option<String> {
    let node = server_names.last()?;
    for word in &node.args {
        if word.dynamic {
            projection.notices.push(ConfigNotice {
                line: node.line,
                section: section.to_owned(),
                message: format!(
                    "dynamic server_name `{}` was preserved but not expanded",
                    word.text
                ),
            });
        } else if word.text != "_" && !word.text.is_empty() && !word.text.starts_with(['~', '*']) {
            return Some(word.text.clone());
        } else if word.text.starts_with(['~', '*']) {
            projection.notices.push(ConfigNotice {
                line: node.line,
                section: section.to_owned(),
                message: format!(
                    "non-literal server_name `{}` was preserved but not used as an endpoint hostname",
                    word.text
                ),
            });
        }
    }
    None
}

fn endpoint_name(hostname: Option<&str>, listens: &[&Node], context: &Context<'_>) -> String {
    if let Some(hostname) = hostname {
        return hostname.to_owned();
    }
    if let Some(listen) = listens.first().and_then(|node| node.args.first())
        && !listen.dynamic
    {
        return listen.text.clone();
    }
    format!("nginx-server@{}", context.section)
}

fn project_protocols(
    nodes: &[&Node],
    section: &str,
    projection: &mut Projection,
) -> Vec<ProtocolVersion> {
    let mut protocols = Vec::new();
    for node in nodes {
        for word in &node.args {
            if word.dynamic {
                projection.notices.push(ConfigNotice {
                    line: node.line,
                    section: section.to_owned(),
                    message: format!(
                        "dynamic ssl_protocols value `{}` was preserved but not expanded",
                        word.text
                    ),
                });
                continue;
            }
            match word.text.parse() {
                Ok(protocol) => protocols.push(protocol),
                Err(message) => projection.notices.push(ConfigNotice {
                    line: node.line,
                    section: section.to_owned(),
                    message: format!("{message}; value preserved"),
                }),
            }
        }
    }
    protocols.sort_unstable();
    protocols.dedup();
    protocols
}

fn project_colon_values<'a>(
    nodes: impl Iterator<Item = &'a Node>,
    section: &str,
    projection: &mut Projection,
) -> Vec<String> {
    let mut values = Vec::new();
    for node in nodes {
        for word in &node.args {
            if word.dynamic {
                projection.notices.push(ConfigNotice {
                    line: node.line,
                    section: section.to_owned(),
                    message: format!(
                        "dynamic {} value `{}` was preserved but not expanded",
                        node.name, word.text
                    ),
                });
                continue;
            }
            for value in word.text.split(':').filter(|value| !value.is_empty()) {
                if is_openssl_selector(value) {
                    projection.notices.push(ConfigNotice {
                        line: node.line,
                        section: section.to_owned(),
                        message: format!(
                            "cipher selector `{value}` is retained as configured evidence; the adapter does not resolve the OpenSSL cipher database"
                        ),
                    });
                } else {
                    values.push(value.to_owned());
                }
            }
        }
    }
    deduplicate(values)
}

fn project_paths(
    nodes: &[&Node],
    directive: &str,
    section: &str,
    projection: &mut Projection,
) -> Vec<String> {
    let mut paths = Vec::new();
    for node in nodes {
        for word in &node.args {
            if word.dynamic {
                projection.notices.push(ConfigNotice {
                    line: node.line,
                    section: section.to_owned(),
                    message: format!(
                        "dynamic {directive} path `{}` was preserved but not expanded",
                        word.text
                    ),
                });
            } else {
                paths.push(word.text.clone());
            }
        }
    }
    paths
}

fn is_hsts(node: &Node) -> bool {
    node.args
        .first()
        .is_some_and(|word| word.text.eq_ignore_ascii_case("strict-transport-security"))
}

fn project_hsts(node: &Node, section: &str, projection: &mut Projection) -> HstsPolicy {
    let mut policy = HstsPolicy {
        enabled: true,
        ..HstsPolicy::default()
    };
    let Some(value) = node.args.get(1) else {
        projection.notices.push(ConfigNotice {
            line: node.line,
            section: section.to_owned(),
            message: "HSTS add_header has no header value".to_owned(),
        });
        return policy;
    };
    if value.dynamic {
        projection.notices.push(ConfigNotice {
            line: node.line,
            section: section.to_owned(),
            message: "dynamic HSTS header was preserved but not expanded".to_owned(),
        });
    }
    for component in value.text.split(';').map(str::trim) {
        let lower = component.to_ascii_lowercase();
        if let Some(seconds) = lower.strip_prefix("max-age=") {
            match seconds.parse() {
                Ok(seconds) => policy.max_age = Some(seconds),
                Err(_) => projection.notices.push(ConfigNotice {
                    line: node.line,
                    section: section.to_owned(),
                    message: format!("invalid HSTS `{component}` was preserved"),
                }),
            }
        } else if lower == "includesubdomains" {
            policy.include_subdomains = true;
        } else if lower == "preload" {
            policy.preload = true;
        } else if !component.is_empty() {
            projection.notices.push(ConfigNotice {
                line: node.line,
                section: section.to_owned(),
                message: format!("unknown HSTS component `{component}` was preserved"),
            });
        }
    }
    policy
}

fn add_endpoint_extensions(
    endpoint: &mut Endpoint,
    server_nodes: &[Node],
    listens: &[&Node],
    server_names: &[&Node],
    keys: &[&Node],
) {
    extend_nodes(&mut endpoint.extensions, "nginx.listen", listens);
    extend_nodes(&mut endpoint.extensions, "nginx.server_name", server_names);
    extend_nodes(&mut endpoint.extensions, "nginx.ssl_certificate_key", keys);
    for node in server_nodes {
        if !is_projected_server_node(node) {
            endpoint
                .extensions
                .entry(format!("nginx.{}", node.name))
                .or_default()
                .push(node_value(node));
        } else if node.name.eq_ignore_ascii_case("add_header") && !is_hsts(node) {
            endpoint
                .extensions
                .entry("nginx.add_header".to_owned())
                .or_default()
                .push(node_value(node));
        }
    }
}

fn extend_nodes(extensions: &mut BTreeMap<String, Vec<TlsValue>>, key: &str, nodes: &[&Node]) {
    for node in nodes {
        extensions
            .entry(key.to_owned())
            .or_default()
            .push(node_value(node));
    }
}

fn is_projected_server_node(node: &Node) -> bool {
    matches!(
        node.name.to_ascii_lowercase().as_str(),
        "ssl_protocols"
            | "ssl_ciphers"
            | "ssl_ciphersuites"
            | "ssl_certificate"
            | "ssl_certificate_key"
            | "server_name"
            | "listen"
            | "add_header"
    )
}

fn collect_notices(nodes: &[Node], section: &str, notices: &mut Vec<ConfigNotice>) {
    for node in nodes {
        let nested_section = format!("{section}/{}@{}", node.name, node.line);
        if node.name.eq_ignore_ascii_case("include") {
            notices.push(ConfigNotice {
                line: node.line,
                section: section.to_owned(),
                message: format!(
                    "include `{}` was preserved but not followed",
                    node.values().join(" ")
                ),
            });
        } else if node.children.is_none() && !is_known_node(node) {
            notices.push(ConfigNotice {
                line: node.line,
                section: section.to_owned(),
                message: format!("unsupported nginx directive `{}` was preserved", node.name),
            });
        }
        if let Some(children) = &node.children {
            collect_notices(children, &nested_section, notices);
        }
    }
}

fn is_known_node(node: &Node) -> bool {
    is_projected_server_node(node)
        || matches!(
            node.name.to_ascii_lowercase().as_str(),
            "include" | "http" | "stream" | "server" | "events"
        )
}

fn contains_tls_state(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| {
        node.name.eq_ignore_ascii_case("ssl")
            || node.name.to_ascii_lowercase().starts_with("ssl_")
            || (node.name.eq_ignore_ascii_case("add_header") && is_hsts(node))
    })
}

fn server_uses_tls(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| {
        node.name.eq_ignore_ascii_case("ssl")
            || node.name.to_ascii_lowercase().starts_with("ssl_")
            || (node.name.eq_ignore_ascii_case("listen")
                && node.args.iter().any(|argument| {
                    argument.text.eq_ignore_ascii_case("ssl")
                        || argument.text.eq_ignore_ascii_case("quic")
                }))
    })
}

fn main_extensions(nodes: &[Node]) -> BTreeMap<String, Vec<TlsValue>> {
    let mut extensions = BTreeMap::new();
    for node in nodes {
        if !matches!(
            node.name.to_ascii_lowercase().as_str(),
            "http" | "stream" | "server"
        ) {
            extensions
                .entry(format!("nginx.{}", node.name))
                .or_insert_with(Vec::new)
                .push(node_value(node));
        }
    }
    extensions
}

fn node_value(node: &Node) -> TlsValue {
    if let Some(children) = &node.children {
        let mut fields = BTreeMap::new();
        if !node.args.is_empty() {
            fields.insert("_arguments".to_owned(), vec![words_value(&node.args)]);
        }
        for child in children {
            fields
                .entry(child.name.clone())
                .or_insert_with(Vec::new)
                .push(node_value(child));
        }
        TlsValue::Object(fields)
    } else {
        words_value(&node.args)
    }
}

fn words_value(words: &[Word]) -> TlsValue {
    match words {
        [word] => TlsValue::Scalar(word.text.clone()),
        _ => TlsValue::List(words.iter().map(|word| word.text.clone()).collect()),
    }
}

fn deduplicate(values: Vec<String>) -> Vec<String> {
    let mut output = Vec::new();
    for value in values {
        if !output.contains(&value) {
            output.push(value);
        }
    }
    output
}

fn is_openssl_selector(value: &str) -> bool {
    value.starts_with(['!', '-', '+', '@'])
        || [
            "DEFAULT",
            "ALL",
            "COMPLEMENTOFDEFAULT",
            "COMPLEMENTOFALL",
            "HIGH",
            "MEDIUM",
            "LOW",
        ]
        .iter()
        .any(|selector| value.eq_ignore_ascii_case(selector))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unclosed_blocks_and_quotes() {
        assert!(import_nginx("server {").is_err());
        assert!(import_nginx("server_name \"example.test;").is_err());
    }

    #[test]
    fn never_follows_include_or_expands_variables() {
        let imported = import_nginx(
            "include /missing/*.conf; server { listen $port ssl; server_name $host; }",
        )
        .expect("configuration is structurally valid");
        assert_eq!(imported.endpoints.len(), 1);
        assert_eq!(imported.endpoints[0].hostname, None);
        assert!(
            imported
                .notices
                .iter()
                .any(|notice| notice.message.contains("not followed"))
        );
        assert_eq!(
            imported.raw.lines().next(),
            Some("include /missing/*.conf; server { listen $port ssl; server_name $host; }")
        );
    }

    #[test]
    fn plain_http_server_is_not_projected_as_tls() {
        let imported = import_nginx(
            "http { server { listen 80; server_name plain.example; } server { listen 443 ssl; server_name tls.example; } }",
        )
        .expect("configuration is structurally valid");
        assert_eq!(imported.endpoints.len(), 1);
        assert_eq!(
            imported.endpoints[0].hostname.as_deref(),
            Some("tls.example")
        );
    }
}
