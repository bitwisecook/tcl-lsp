//! Render an [`XCTranslationResult`] as Terraform HCL (volterra provider):
//! `volterra_origin_pool`, `volterra_http_loadbalancer`, and
//! `volterra_service_policy` resources. Hand-rolled string formatting — a
//! faithful port of `dialects/f5/xc/terraform.py`.

use crate::model::{
    TranslateStatus, XCCookieMatch, XCHeaderAction, XCHeaderMatch, XCOriginPool, XCPathMatch,
    XCQueryMatch, XCRoute, XCServicePolicy, XCTranslationResult, XCWafExclusionRule,
};

/// Escape and quote a Terraform string value.
fn quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// `2 * level` spaces of indentation.
fn pad(level: usize) -> String {
    "  ".repeat(level)
}

fn path_match_block(m: &XCPathMatch, level: usize) -> String {
    let p = pad(level);
    let mut lines = vec![format!("{p}path {{")];
    lines.push(format!("{p}  {} = {}", m.match_type, quote(&m.value)));
    if m.invert {
        lines.push(format!("{p}  invert_matcher = true"));
    }
    lines.push(format!("{p}}}"));
    lines.join("\n")
}

fn header_match_block(m: &XCHeaderMatch, level: usize) -> String {
    let p = pad(level);
    let mut lines = vec![format!("{p}headers {{")];
    lines.push(format!("{p}  name = {}", quote(&m.name)));
    if m.match_type == "presence" {
        lines.push(format!("{p}  presence = true"));
    } else {
        lines.push(format!("{p}  {} = {}", m.match_type, quote(&m.value)));
    }
    if m.invert {
        lines.push(format!("{p}  invert_matcher = true"));
    }
    lines.push(format!("{p}}}"));
    lines.join("\n")
}

fn cookie_match_block(m: &XCCookieMatch, level: usize) -> String {
    let p = pad(level);
    let mut lines = vec![format!("{p}cookies {{")];
    lines.push(format!("{p}  name = {}", quote(&m.name)));
    if m.match_type == "presence" {
        lines.push(format!("{p}  presence = true"));
    } else {
        lines.push(format!("{p}  {} = {}", m.match_type, quote(&m.value)));
    }
    if m.invert {
        lines.push(format!("{p}  invert_matcher = true"));
    }
    lines.push(format!("{p}}}"));
    lines.join("\n")
}

fn query_match_block(m: &XCQueryMatch, level: usize) -> String {
    let p = pad(level);
    let mut lines = vec![format!("{p}query_params {{")];
    lines.push(format!("{p}  {} = {}", m.match_type, quote(&m.value)));
    lines.push(format!("{p}}}"));
    lines.join("\n")
}

fn render_origin_pool(pool: &XCOriginPool, namespace: &str) -> String {
    [
        format!("resource \"volterra_origin_pool\" {} {{", quote(&pool.name)),
        format!("  name      = {}", quote(&pool.name)),
        format!("  namespace = {}", quote(namespace)),
        String::new(),
        "  # TODO: Configure origin servers".to_owned(),
        "  origin_servers {".to_owned(),
        "    public_name {".to_owned(),
        "      dns_name = \"example.com\"  # TODO: Set actual server address".to_owned(),
        "    }".to_owned(),
        "  }".to_owned(),
        String::new(),
        format!("  port                  = {}", pool.port),
        "  endpoint_selection     = \"LOCAL_PREFERRED\"".to_owned(),
        "  loadbalancer_algorithm = \"LB_OVERRIDE\"".to_owned(),
        "}".to_owned(),
    ]
    .join("\n")
}

fn render_route(route: &XCRoute, level: usize) -> String {
    if route.redirect.is_some() {
        render_redirect_route(route, level)
    } else if route.direct_response.is_some() {
        render_direct_response_route(route, level)
    } else {
        render_simple_route(route, level)
    }
}

fn render_simple_route(route: &XCRoute, level: usize) -> String {
    let p = pad(level);
    let mut lines = vec![format!("{p}simple_route {{")];

    if let Some(pm) = &route.path_match {
        lines.push(path_match_block(pm, level + 1));
    }
    for hdr in &route.header_matches {
        lines.push(header_match_block(hdr, level + 1));
    }
    if let Some(qm) = &route.query_match {
        lines.push(query_match_block(qm, level + 1));
    }
    for cookie in &route.cookie_matches {
        lines.push(cookie_match_block(cookie, level + 1));
    }

    if let Some(op) = &route.origin_pool {
        lines.push(format!("{p}  origin_pools {{"));
        lines.push(format!("{p}    pool {{"));
        lines.push(format!("{p}      name      = volterra_origin_pool.{}.name", op.name));
        lines.push(format!("{p}      namespace = volterra_origin_pool.{}.namespace", op.name));
        lines.push(format!("{p}    }}"));
        lines.push(format!("{p}  }}"));
    }

    for action in &route.header_actions {
        let target = if action.target == "request" { "request" } else { "response" };
        if action.operation == "remove" {
            lines.push(format!("{p}  {target}_headers_to_remove = [{}]", quote(&action.name)));
        } else {
            lines.push(format!("{p}  {target}_headers_to_add {{"));
            lines.push(format!("{p}    name  = {}", quote(&action.name)));
            lines.push(format!("{p}    value = {}", quote(&action.value)));
            if action.operation == "replace" {
                lines.push(format!("{p}    append = false"));
            }
            lines.push(format!("{p}  }}"));
        }
    }

    lines.push(format!("{p}}}"));
    lines.join("\n")
}

fn render_redirect_route(route: &XCRoute, level: usize) -> String {
    let p = pad(level);
    let mut lines = vec![format!("{p}redirect_route {{")];

    if let Some(pm) = &route.path_match {
        lines.push(path_match_block(pm, level + 1));
    }

    lines.push(format!("{p}  redirect_route_action {{"));
    if let Some(redirect) = &route.redirect {
        lines.push(format!("{p}    host_redirect = {}", quote(&redirect.url)));
        if redirect.response_code == 301 {
            lines.push(format!("{p}    response_code = \"MOVED_PERMANENTLY\""));
        } else {
            lines.push(format!("{p}    response_code = \"FOUND\""));
        }
    }
    lines.push(format!("{p}  }}"));

    lines.push(format!("{p}}}"));
    lines.join("\n")
}

fn render_direct_response_route(route: &XCRoute, level: usize) -> String {
    let p = pad(level);
    let mut lines = vec![format!("{p}direct_response_route {{")];

    if let Some(pm) = &route.path_match {
        lines.push(path_match_block(pm, level + 1));
    }

    if let Some(dr) = &route.direct_response {
        lines.push(format!("{p}  direct_response_action {{"));
        lines.push(format!("{p}    status = \"{}\"", dr.status_code));
        if !dr.body.is_empty() {
            lines.push(format!("{p}    content = {}", quote(&dr.body)));
        }
        lines.push(format!("{p}  }}"));
    }

    lines.push(format!("{p}}}"));
    lines.join("\n")
}

fn render_header_actions(actions: &[XCHeaderAction], level: usize) -> String {
    let p = pad(level);
    let mut lines: Vec<String> = Vec::new();
    for action in actions {
        if action.operation == "remove" {
            lines.push(format!("{p}{}_headers_to_remove = [{}]", action.target, quote(&action.name)));
        } else {
            lines.push(format!("{p}{}_headers_to_add {{", action.target));
            lines.push(format!("{p}  name  = {}", quote(&action.name)));
            if !action.value.is_empty() {
                lines.push(format!("{p}  value = {}", quote(&action.value)));
            }
            if action.operation == "replace" {
                lines.push(format!("{p}  append = false"));
            }
            lines.push(format!("{p}}}"));
        }
    }
    lines.join("\n")
}

fn render_service_policy(policy: &XCServicePolicy, namespace: &str) -> String {
    let mut lines = vec![
        format!("resource \"volterra_service_policy\" {} {{", quote(&policy.name)),
        format!("  name      = {}", quote(&policy.name)),
        format!("  namespace = {}", quote(namespace)),
        format!("  algo      = \"{}\"", policy.algo),
        String::new(),
        "  rule_list {".to_owned(),
    ];

    for rule in &policy.rules {
        lines.push("    rules {".to_owned());
        if !rule.name.is_empty() {
            lines.push("      metadata {".to_owned());
            lines.push(format!("        name = {}", quote(&rule.name)));
            if !rule.description.is_empty() {
                lines.push(format!("        description = {}", quote(&rule.description)));
            }
            lines.push("      }".to_owned());
        }
        lines.push("      spec {".to_owned());

        // Action
        match rule.action.as_str() {
            "deny" => lines.push("        action = \"DENY\"".to_owned()),
            "allow" => lines.push("        action = \"ALLOW\"".to_owned()),
            _ => lines.push("        action = \"NEXT_POLICY\"".to_owned()),
        }

        // Match criteria
        if let Some(pm) = &rule.path_match {
            lines.push(path_match_block(pm, 4));
        }
        if let Some(hm) = &rule.host_match {
            lines.push("        host {".to_owned());
            lines.push(format!("          {} = {}", hm.match_type, quote(&hm.value)));
            lines.push("        }".to_owned());
        }
        if let Some(mm) = &rule.method_match {
            lines.push("        http_method {".to_owned());
            let methods = mm.methods.iter().map(|m| quote(m)).collect::<Vec<_>>().join(", ");
            lines.push(format!("          methods = [{methods}]"));
            if mm.invert {
                lines.push("          invert_matcher = true".to_owned());
            }
            lines.push("        }".to_owned());
        }
        for hdr in &rule.header_matches {
            lines.push(header_match_block(hdr, 4));
        }
        if let Some(qm) = &rule.query_match {
            lines.push(query_match_block(qm, 4));
        }
        for cookie in &rule.cookie_matches {
            lines.push(cookie_match_block(cookie, 4));
        }
        if !rule.ip_prefix_list.is_empty() {
            lines.push("        client_source {".to_owned());
            lines.push("          ip_prefix_list {".to_owned());
            let prefixes = rule.ip_prefix_list.iter().map(|ip| quote(ip)).collect::<Vec<_>>().join(", ");
            lines.push(format!("            prefixes = [{prefixes}]"));
            lines.push("          }".to_owned());
            lines.push("        }".to_owned());
        }

        lines.push("      }".to_owned());
        lines.push("    }".to_owned());
    }

    lines.push("  }".to_owned());
    lines.push("}".to_owned());
    lines.join("\n")
}

fn render_waf_exclusion_rule(rule: &XCWafExclusionRule, level: usize) -> String {
    let p = pad(level);
    let mut lines = vec![format!("{p}waf_exclusion_rules {{")];
    lines.push(format!("{p}  metadata {{"));
    lines.push(format!("{p}    name = {}", quote(&rule.name)));
    if !rule.description.is_empty() {
        lines.push(format!("{p}    description = {}", quote(&rule.description)));
    }
    lines.push(format!("{p}  }}"));

    if rule.path_match.is_some() || rule.ip_prefix_set.is_some() {
        lines.push(format!("{p}  match_condition {{"));
        if let Some(pm) = &rule.path_match {
            lines.push(path_match_block(pm, level + 2));
        }
        if let Some(ip_set) = &rule.ip_prefix_set {
            lines.push(format!("{p}    client_source {{"));
            lines.push(format!("{p}      ip_prefix_set {{"));
            lines.push(format!("{p}        ref = [{}]", quote(ip_set)));
            lines.push(format!("{p}      }}"));
            lines.push(format!("{p}    }}"));
        }
        lines.push(format!("{p}  }}"));
    }

    lines.push(format!("{p}  app_firewall_detection_control {{"));
    lines.push(format!("{p}    exclude_all_attack_types = true"));
    lines.push(format!("{p}  }}"));
    lines.push(format!("{p}}}"));
    lines.join("\n")
}

fn render_load_balancer(result: &XCTranslationResult, namespace: &str, name: &str) -> String {
    let mut lines = vec![
        format!("resource \"volterra_http_loadbalancer\" {} {{", quote(name)),
        format!("  name      = {}", quote(name)),
        format!("  namespace = {}", quote(namespace)),
        String::new(),
        "  # TODO: Configure domains and advertise policy".to_owned(),
        "  domains = [\"example.com\"]  # TODO: Set actual domains".to_owned(),
        String::new(),
        "  http {".to_owned(),
        "    dns_volterra_managed = false".to_owned(),
        "  }".to_owned(),
    ];

    if !result.routes.is_empty() {
        lines.push(String::new());
        lines.push("  # --- Routes ---".to_owned());
        for route in &result.routes {
            lines.push(render_route(route, 1));
        }
    }

    if !result.header_actions.is_empty() {
        lines.push(String::new());
        lines.push("  # --- Header Processing ---".to_owned());
        lines.push(render_header_actions(&result.header_actions, 1));
    }

    if !result.waf_exclusion_rules.is_empty() {
        lines.push(String::new());
        lines.push("  # --- WAF Exclusion Rules ---".to_owned());
        for rule in &result.waf_exclusion_rules {
            lines.push(render_waf_exclusion_rule(rule, 1));
        }
    }

    if !result.service_policies.is_empty() {
        lines.push(String::new());
        lines.push("  active_service_policies {".to_owned());
        for policy in &result.service_policies {
            lines.push("    policies {".to_owned());
            lines.push(format!("      name      = volterra_service_policy.{}.name", policy.name));
            lines.push(format!("      namespace = volterra_service_policy.{}.namespace", policy.name));
            lines.push("    }".to_owned());
        }
        lines.push("  }".to_owned());
    }

    lines.push("}".to_owned());
    lines.join("\n")
}

fn render_summary(result: &XCTranslationResult) -> String {
    let mut lines = vec![
        "# iRule → F5 XC Translation".to_owned(),
        format!("# Coverage: {:.1}%", result.coverage_pct),
        format!(
            "# Translated: {}  |  Partial: {}  |  Untranslatable: {}",
            result.translatable_count(),
            result.partial_count(),
            result.untranslatable_count(),
        ),
    ];

    let untranslatable: Vec<_> =
        result.items.iter().filter(|i| i.status == TranslateStatus::Untranslatable).collect();
    if !untranslatable.is_empty() {
        lines.push("#".to_owned());
        lines.push("# Untranslatable constructs:".to_owned());
        for item in untranslatable {
            lines.push(format!("#   - {}: {}", item.irule_command, item.xc_description));
            if !item.note.is_empty() {
                lines.push(format!("#     {}", item.note));
            }
        }
    }

    let advisory: Vec<_> =
        result.items.iter().filter(|i| i.status == TranslateStatus::Advisory).collect();
    if !advisory.is_empty() {
        lines.push("#".to_owned());
        lines.push("# Advisory (separate XC features):".to_owned());
        for item in advisory {
            lines.push(format!("#   - {}: {}", item.irule_command, item.xc_description));
        }
    }

    lines.push(String::new());
    lines.join("\n")
}

/// Render an XC translation result as Terraform HCL.
#[must_use]
pub fn render_terraform(result: &XCTranslationResult, namespace: &str, lb_name: &str) -> String {
    let mut sections: Vec<String> = vec![render_summary(result)];

    sections.push(
        "terraform {\n  required_providers {\n    volterra = {\n      source  = \
         \"volterraedge/volterra\"\n      version = \">= 0.11.47\"\n    }\n  }\n}\n"
            .to_owned(),
    );

    for pool in &result.origin_pools {
        sections.push(render_origin_pool(pool, namespace));
    }
    for policy in &result.service_policies {
        sections.push(render_service_policy(policy, namespace));
    }
    sections.push(render_load_balancer(result, namespace, lb_name));

    format!("{}\n", sections.join("\n\n"))
}
