"""Render XC translation results as Terraform HCL (volterra provider).

Generates ``volterra_origin_pool``, ``volterra_http_loadbalancer``, and
``volterra_service_policy`` resources.  Hand-rolled string formatting —
no HCL library needed.
"""

from __future__ import annotations

from .xc_model import (
    TranslateStatus,
    XCCookieMatch,
    XCHeaderAction,
    XCHeaderMatch,
    XCOriginPool,
    XCPathMatch,
    XCQueryMatch,
    XCRoute,
    XCServicePolicy,
    XCTranslationResult,
    XCWafExclusionRule,
)


def _indent(text: str, level: int) -> str:
    return "\n".join(("  " * level) + line for line in text.splitlines())


def _quote(value: str) -> str:
    """Escape and quote a Terraform string value."""
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def _path_match_block(match: XCPathMatch, indent_level: int) -> str:
    prefix = "  " * indent_level
    lines: list[str] = [f"{prefix}path {{"]
    lines.append(f"{prefix}  {match.match_type} = {_quote(match.value)}")
    if match.invert:
        lines.append(f"{prefix}  invert_matcher = true")
    lines.append(f"{prefix}}}")
    return "\n".join(lines)


def _header_match_block(match: XCHeaderMatch, indent_level: int) -> str:
    prefix = "  " * indent_level
    lines: list[str] = [f"{prefix}headers {{"]
    lines.append(f"{prefix}  name = {_quote(match.name)}")
    if match.match_type == "presence":
        lines.append(f"{prefix}  presence = true")
    else:
        lines.append(f"{prefix}  {match.match_type} = {_quote(match.value)}")
    if match.invert:
        lines.append(f"{prefix}  invert_matcher = true")
    lines.append(f"{prefix}}}")
    return "\n".join(lines)


def _cookie_match_block(match: XCCookieMatch, indent_level: int) -> str:
    prefix = "  " * indent_level
    lines: list[str] = [f"{prefix}cookies {{"]
    lines.append(f"{prefix}  name = {_quote(match.name)}")
    if match.match_type == "presence":
        lines.append(f"{prefix}  presence = true")
    else:
        lines.append(f"{prefix}  {match.match_type} = {_quote(match.value)}")
    if match.invert:
        lines.append(f"{prefix}  invert_matcher = true")
    lines.append(f"{prefix}}}")
    return "\n".join(lines)


def _query_match_block(match: XCQueryMatch, indent_level: int) -> str:
    prefix = "  " * indent_level
    lines: list[str] = [f"{prefix}query_params {{"]
    lines.append(f"{prefix}  {match.match_type} = {_quote(match.value)}")
    lines.append(f"{prefix}}}")
    return "\n".join(lines)


def _render_origin_pool(pool: XCOriginPool, namespace: str) -> str:
    lines: list[str] = [
        f'resource "volterra_origin_pool" {_quote(pool.name)} {{',
        f"  name      = {_quote(pool.name)}",
        f"  namespace = {_quote(namespace)}",
        "",
        "  # TODO: Configure origin servers",
        "  origin_servers {",
        "    public_name {",
        '      dns_name = "example.com"  # TODO: Set actual server address',
        "    }",
        "  }",
        "",
        f"  port                  = {pool.port}",
        '  endpoint_selection     = "LOCAL_PREFERRED"',
        '  loadbalancer_algorithm = "LB_OVERRIDE"',
        "}",
    ]
    return "\n".join(lines)


def _render_route(route: XCRoute, indent_level: int) -> str:
    if route.redirect:
        return _render_redirect_route(route, indent_level)
    elif route.direct_response:
        return _render_direct_response_route(route, indent_level)
    else:
        return _render_simple_route(route, indent_level)


def _render_simple_route(route: XCRoute, indent_level: int) -> str:
    prefix = "  " * indent_level
    lines: list[str] = [f"{prefix}simple_route {{"]

    if route.path_match:
        lines.append(_path_match_block(route.path_match, indent_level + 1))

    for hdr in route.header_matches:
        lines.append(_header_match_block(hdr, indent_level + 1))

    if route.query_match:
        lines.append(_query_match_block(route.query_match, indent_level + 1))

    for cookie in route.cookie_matches:
        lines.append(_cookie_match_block(cookie, indent_level + 1))

    if route.origin_pool:
        lines.append(f"{prefix}  origin_pools {{")
        lines.append(f"{prefix}    pool {{")
        lines.append(
            f"{prefix}      name      = volterra_origin_pool.{route.origin_pool.name}.name"
        )
        lines.append(
            f"{prefix}      namespace = volterra_origin_pool.{route.origin_pool.name}.namespace"
        )
        lines.append(f"{prefix}    }}")
        lines.append(f"{prefix}  }}")

    for action in route.header_actions:
        target = "request" if action.target == "request" else "response"
        if action.operation == "remove":
            lines.append(f"{prefix}  {target}_headers_to_remove = [{_quote(action.name)}]")
        else:
            lines.append(f"{prefix}  {target}_headers_to_add {{")
            lines.append(f"{prefix}    name  = {_quote(action.name)}")
            lines.append(f"{prefix}    value = {_quote(action.value)}")
            if action.operation == "replace":
                lines.append(f"{prefix}    append = false")
            lines.append(f"{prefix}  }}")

    lines.append(f"{prefix}}}")
    return "\n".join(lines)


def _render_redirect_route(route: XCRoute, indent_level: int) -> str:
    prefix = "  " * indent_level
    lines: list[str] = [f"{prefix}redirect_route {{"]

    if route.path_match:
        lines.append(_path_match_block(route.path_match, indent_level + 1))

    lines.append(f"{prefix}  redirect_route_action {{")
    if route.redirect:
        lines.append(f"{prefix}    host_redirect = {_quote(route.redirect.url)}")
        code = route.redirect.response_code
        if code == 301:
            lines.append(f'{prefix}    response_code = "MOVED_PERMANENTLY"')
        else:
            lines.append(f'{prefix}    response_code = "FOUND"')
    lines.append(f"{prefix}  }}")

    lines.append(f"{prefix}}}")
    return "\n".join(lines)


def _render_direct_response_route(route: XCRoute, indent_level: int) -> str:
    prefix = "  " * indent_level
    lines: list[str] = [f"{prefix}direct_response_route {{"]

    if route.path_match:
        lines.append(_path_match_block(route.path_match, indent_level + 1))

    if route.direct_response:
        lines.append(f"{prefix}  direct_response_action {{")
        lines.append(f'{prefix}    status = "{route.direct_response.status_code}"')
        if route.direct_response.body:
            lines.append(f"{prefix}    content = {_quote(route.direct_response.body)}")
        lines.append(f"{prefix}  }}")

    lines.append(f"{prefix}}}")
    return "\n".join(lines)


def _render_header_actions(actions: tuple[XCHeaderAction, ...], indent_level: int) -> str:
    prefix = "  " * indent_level
    lines: list[str] = []
    for action in actions:
        if action.operation == "remove":
            lines.append(f"{prefix}{action.target}_headers_to_remove = [{_quote(action.name)}]")
        else:
            lines.append(f"{prefix}{action.target}_headers_to_add {{")
            lines.append(f"{prefix}  name  = {_quote(action.name)}")
            if action.value:
                lines.append(f"{prefix}  value = {_quote(action.value)}")
            if action.operation == "replace":
                lines.append(f"{prefix}  append = false")
            lines.append(f"{prefix}}}")
    return "\n".join(lines)


def _render_service_policy(policy: XCServicePolicy, namespace: str) -> str:
    lines: list[str] = [
        f'resource "volterra_service_policy" {_quote(policy.name)} {{',
        f"  name      = {_quote(policy.name)}",
        f"  namespace = {_quote(namespace)}",
        f'  algo      = "{policy.algo}"',
        "",
        "  rule_list {",
    ]

    for rule in policy.rules:
        lines.append("    rules {")
        if rule.name:
            lines.append("      metadata {")
            lines.append(f"        name = {_quote(rule.name)}")
            if rule.description:
                lines.append(f"        description = {_quote(rule.description)}")
            lines.append("      }")
        lines.append("      spec {")

        # Action
        if rule.action == "deny":
            lines.append('        action = "DENY"')
        elif rule.action == "allow":
            lines.append('        action = "ALLOW"')
        else:
            lines.append('        action = "NEXT_POLICY"')

        # Match criteria
        if rule.path_match:
            lines.append(_path_match_block(rule.path_match, indent_level=4))

        if rule.host_match:
            lines.append("        host {")
            lines.append(
                f"          {rule.host_match.match_type} = {_quote(rule.host_match.value)}"
            )
            lines.append("        }")

        if rule.method_match:
            lines.append("        http_method {")
            lines.append(
                f"          methods = [{', '.join(_quote(m) for m in rule.method_match.methods)}]"
            )
            if rule.method_match.invert:
                lines.append("          invert_matcher = true")
            lines.append("        }")

        for hdr in rule.header_matches:
            lines.append(_header_match_block(hdr, indent_level=4))

        if rule.query_match:
            lines.append(_query_match_block(rule.query_match, indent_level=4))

        for cookie in rule.cookie_matches:
            lines.append(_cookie_match_block(cookie, indent_level=4))

        if rule.ip_prefix_list:
            lines.append("        client_source {")
            lines.append("          ip_prefix_list {")
            lines.append(
                f"            prefixes = [{', '.join(_quote(ip) for ip in rule.ip_prefix_list)}]"
            )
            lines.append("          }")
            lines.append("        }")

        lines.append("      }")
        lines.append("    }")

    lines.append("  }")
    lines.append("}")
    return "\n".join(lines)


def _render_waf_exclusion_rule(rule: XCWafExclusionRule, indent_level: int) -> str:
    prefix = "  " * indent_level
    lines: list[str] = [f"{prefix}waf_exclusion_rules {{"]
    lines.append(f"{prefix}  metadata {{")
    lines.append(f"{prefix}    name = {_quote(rule.name)}")
    if rule.description:
        lines.append(f"{prefix}    description = {_quote(rule.description)}")
    lines.append(f"{prefix}  }}")

    if rule.path_match or rule.ip_prefix_set:
        lines.append(f"{prefix}  match_condition {{")
        if rule.path_match:
            lines.append(_path_match_block(rule.path_match, indent_level + 2))
        if rule.ip_prefix_set:
            lines.append(f"{prefix}    client_source {{")
            lines.append(f"{prefix}      ip_prefix_set {{")
            lines.append(f"{prefix}        ref = [{_quote(rule.ip_prefix_set)}]")
            lines.append(f"{prefix}      }}")
            lines.append(f"{prefix}    }}")
        lines.append(f"{prefix}  }}")

    lines.append(f"{prefix}  app_firewall_detection_control {{")
    lines.append(f"{prefix}    exclude_all_attack_types = true")
    lines.append(f"{prefix}  }}")
    lines.append(f"{prefix}}}")
    return "\n".join(lines)


def _render_load_balancer(
    result: XCTranslationResult,
    namespace: str,
    name: str = "translated-lb",
) -> str:
    lines: list[str] = [
        f'resource "volterra_http_loadbalancer" {_quote(name)} {{',
        f"  name      = {_quote(name)}",
        f"  namespace = {_quote(namespace)}",
        "",
        "  # TODO: Configure domains and advertise policy",
        '  domains = ["example.com"]  # TODO: Set actual domains',
        "",
        "  http {",
        "    dns_volterra_managed = false",
        "  }",
    ]

    # Routes
    if result.routes:
        lines.append("")
        lines.append("  # --- Routes ---")
        for route in result.routes:
            lines.append(_render_route(route, indent_level=1))

    # LB-level header actions
    if result.header_actions:
        lines.append("")
        lines.append("  # --- Header Processing ---")
        lines.append(_render_header_actions(result.header_actions, indent_level=1))

    # WAF exclusion rules
    if result.waf_exclusion_rules:
        lines.append("")
        lines.append("  # --- WAF Exclusion Rules ---")
        for rule in result.waf_exclusion_rules:
            lines.append(_render_waf_exclusion_rule(rule, indent_level=1))

    # Service policy reference
    if result.service_policies:
        lines.append("")
        lines.append("  active_service_policies {")
        for policy in result.service_policies:
            lines.append("    policies {")
            lines.append(f"      name      = volterra_service_policy.{policy.name}.name")
            lines.append(f"      namespace = volterra_service_policy.{policy.name}.namespace")
            lines.append("    }")
        lines.append("  }")

    lines.append("}")
    return "\n".join(lines)


# Summary comment


def _render_summary(result: XCTranslationResult) -> str:
    lines: list[str] = [
        "# iRule → F5 XC Translation",
        f"# Coverage: {result.coverage_pct:.1f}%",
        f"# Translated: {result.translatable_count}  |  "
        f"Partial: {result.partial_count}  |  "
        f"Untranslatable: {result.untranslatable_count}",
    ]

    untranslatable = [i for i in result.items if i.status == TranslateStatus.UNTRANSLATABLE]
    if untranslatable:
        lines.append("#")
        lines.append("# Untranslatable constructs:")
        for item in untranslatable:
            lines.append(f"#   - {item.irule_command}: {item.xc_description}")
            if item.note:
                lines.append(f"#     {item.note}")

    advisory = [i for i in result.items if i.status == TranslateStatus.ADVISORY]
    if advisory:
        lines.append("#")
        lines.append("# Advisory (separate XC features):")
        for item in advisory:
            lines.append(f"#   - {item.irule_command}: {item.xc_description}")

    lines.append("")
    return "\n".join(lines)


# Public API


def render_terraform(
    result: XCTranslationResult,
    namespace: str = "default",
    lb_name: str = "translated-lb",
) -> str:
    """Render an XC translation result as Terraform HCL."""
    sections: list[str] = [_render_summary(result)]

    # Provider
    sections.append(
        "terraform {\n"
        "  required_providers {\n"
        "    volterra = {\n"
        '      source  = "volterraedge/volterra"\n'
        '      version = ">= 0.11.47"\n'
        "    }\n"
        "  }\n"
        "}\n"
    )

    # Origin pools
    for pool in result.origin_pools:
        sections.append(_render_origin_pool(pool, namespace))

    # Service policies
    for policy in result.service_policies:
        sections.append(_render_service_policy(policy, namespace))

    # Load balancer
    sections.append(_render_load_balancer(result, namespace, lb_name))

    return "\n\n".join(sections) + "\n"
