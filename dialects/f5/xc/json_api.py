"""Render XC translation results as JSON matching the ves.io API schema.

Produces a Python dict that can be serialised with ``json.dumps()``.
"""

from __future__ import annotations

from typing import Any

from .xc_model import (
    XCCookieMatch,
    XCHeaderAction,
    XCHeaderMatch,
    XCOriginPool,
    XCPathMatch,
    XCQueryMatch,
    XCRoute,
    XCServicePolicy,
    XCServicePolicyRule,
    XCTranslationResult,
    XCWafExclusionRule,
)


def _path_match_dict(match: XCPathMatch) -> dict[str, Any]:
    result: dict[str, Any] = {match.match_type: match.value}
    if match.invert:
        result["invert_matcher"] = True
    return result


def _header_match_dict(match: XCHeaderMatch) -> dict[str, Any]:
    result: dict[str, Any] = {"name": match.name}
    if match.match_type == "presence":
        result["presence"] = True
    else:
        result[match.match_type] = match.value
    if match.invert:
        result["invert_matcher"] = True
    return result


def _cookie_match_dict(match: XCCookieMatch) -> dict[str, Any]:
    result: dict[str, Any] = {"name": match.name}
    if match.match_type == "presence":
        result["presence"] = True
    else:
        result[match.match_type] = match.value
    if match.invert:
        result["invert_matcher"] = True
    return result


def _query_match_dict(match: XCQueryMatch) -> dict[str, str]:
    return {match.match_type: match.value}


def _render_origin_pool(pool: XCOriginPool, namespace: str) -> dict[str, Any]:
    return {
        "metadata": {
            "name": pool.name,
            "namespace": namespace,
        },
        "spec": {
            "origin_servers": [
                {
                    "public_name": {
                        "dns_name": "example.com",  # TODO
                    },
                }
            ],
            "port": pool.port,
            "endpoint_selection": "LOCAL_PREFERRED",
            "loadbalancer_algorithm": "LB_OVERRIDE",
        },
    }


def _render_route(route: XCRoute) -> dict[str, Any]:
    if route.redirect:
        return _render_redirect_route(route)
    elif route.direct_response:
        return _render_direct_response_route(route)
    else:
        return _render_simple_route(route)


def _render_simple_route(route: XCRoute) -> dict[str, Any]:
    result: dict[str, Any] = {"type": "simple_route"}

    match: dict[str, Any] = {}
    if route.path_match:
        match["path"] = _path_match_dict(route.path_match)
    if route.host_match:
        match["host"] = {route.host_match.match_type: route.host_match.value}
    if route.header_matches:
        match["headers"] = [_header_match_dict(h) for h in route.header_matches]
    if route.query_match:
        match["query_params"] = [_query_match_dict(route.query_match)]
    if route.cookie_matches:
        match["cookies"] = [_cookie_match_dict(c) for c in route.cookie_matches]
    if match:
        result["match"] = match

    if route.origin_pool:
        result["origin_pools"] = [{"pool": {"name": route.origin_pool.name}}]

    headers: dict[str, Any] = {}
    for action in route.header_actions:
        key = f"{action.target}_headers_to_{action.operation}"
        if action.operation == "remove":
            headers.setdefault(key, []).append(action.name)
        else:
            headers.setdefault(key, []).append({"name": action.name, "value": action.value})
    if headers:
        result.update(headers)

    return result


def _render_redirect_route(route: XCRoute) -> dict[str, Any]:
    result: dict[str, Any] = {"type": "redirect_route"}

    if route.path_match:
        result["match"] = {"path": _path_match_dict(route.path_match)}

    if route.redirect:
        action: dict[str, Any] = {"host_redirect": route.redirect.url}
        if route.redirect.response_code == 301:
            action["response_code"] = "MOVED_PERMANENTLY"
        else:
            action["response_code"] = "FOUND"
        result["redirect_action"] = action

    return result


def _render_direct_response_route(route: XCRoute) -> dict[str, Any]:
    result: dict[str, Any] = {"type": "direct_response_route"}

    if route.path_match:
        result["match"] = {"path": _path_match_dict(route.path_match)}

    if route.direct_response:
        action: dict[str, Any] = {"status": str(route.direct_response.status_code)}
        if route.direct_response.body:
            action["content"] = route.direct_response.body
        result["direct_response_action"] = action

    return result


def _render_header_action(action: XCHeaderAction) -> dict[str, Any]:
    result: dict[str, Any] = {
        "name": action.name,
        "operation": action.operation,
        "target": action.target,
    }
    if action.value:
        result["value"] = action.value
    return result


def _render_service_policy_rule(rule: XCServicePolicyRule) -> dict[str, Any]:
    rule_spec: dict[str, Any] = {"action": rule.action.upper()}
    rule_metadata: dict[str, Any] = {"name": rule.name}
    result: dict[str, Any] = {
        "metadata": rule_metadata,
        "spec": rule_spec,
    }

    match: dict[str, Any] = {}
    if rule.path_match:
        match["path"] = _path_match_dict(rule.path_match)
    if rule.host_match:
        match["host"] = {rule.host_match.match_type: rule.host_match.value}
    if rule.method_match:
        method_dict: dict[str, Any] = {"methods": list(rule.method_match.methods)}
        if rule.method_match.invert:
            method_dict["invert_matcher"] = True
        match["http_method"] = method_dict
    if rule.header_matches:
        match["headers"] = [_header_match_dict(h) for h in rule.header_matches]
    if rule.query_match:
        match["query_params"] = [_query_match_dict(rule.query_match)]
    if rule.cookie_matches:
        match["cookies"] = [_cookie_match_dict(c) for c in rule.cookie_matches]
    if rule.ip_prefix_list:
        match["client_source"] = {
            "ip_prefix_list": list(rule.ip_prefix_list),
        }
    if match:
        rule_spec["match"] = match

    if rule.description:
        rule_metadata["description"] = rule.description

    return result


def _render_waf_exclusion_rule(rule: XCWafExclusionRule) -> dict[str, Any]:
    result: dict[str, Any] = {
        "metadata": {"name": rule.name},
    }
    if rule.description:
        result["metadata"]["description"] = rule.description

    match_condition: dict[str, Any] = {}
    if rule.path_match:
        match_condition["path"] = _path_match_dict(rule.path_match)
    if rule.ip_prefix_set:
        match_condition["client_source"] = {
            "ip_prefix_set": {"ref": [rule.ip_prefix_set]},
        }
    if match_condition:
        result["match_condition"] = match_condition

    result["app_firewall_detection_control"] = {
        "exclude_all_attack_types": True,
    }
    return result


def _render_service_policy(policy: XCServicePolicy, namespace: str) -> dict[str, Any]:
    return {
        "metadata": {
            "name": policy.name,
            "namespace": namespace,
        },
        "spec": {
            "algo": policy.algo,
            "rule_list": {
                "rules": [_render_service_policy_rule(r) for r in policy.rules],
            },
        },
    }


def _render_load_balancer(
    result: XCTranslationResult,
    namespace: str,
    name: str,
) -> dict[str, Any]:
    lb_spec: dict[str, Any] = {
        "domains": ["example.com"],  # TODO
        "http": {"dns_volterra_managed": False},
    }
    lb: dict[str, Any] = {
        "metadata": {
            "name": name,
            "namespace": namespace,
        },
        "spec": lb_spec,
    }

    if result.routes:
        lb_spec["routes"] = [_render_route(r) for r in result.routes]

    # LB-level header actions
    request_add = [
        a for a in result.header_actions if a.target == "request" and a.operation != "remove"
    ]
    request_remove = [
        a for a in result.header_actions if a.target == "request" and a.operation == "remove"
    ]
    response_add = [
        a for a in result.header_actions if a.target == "response" and a.operation != "remove"
    ]
    response_remove = [
        a for a in result.header_actions if a.target == "response" and a.operation == "remove"
    ]

    if request_add:
        lb_spec["request_headers_to_add"] = [
            {"name": a.name, "value": a.value} for a in request_add
        ]
    if request_remove:
        lb_spec["request_headers_to_remove"] = [a.name for a in request_remove]
    if response_add:
        lb_spec["response_headers_to_add"] = [
            {"name": a.name, "value": a.value} for a in response_add
        ]
    if response_remove:
        lb_spec["response_headers_to_remove"] = [a.name for a in response_remove]

    if result.waf_exclusion_rules:
        lb_spec["waf_exclusion_rules"] = [
            _render_waf_exclusion_rule(r) for r in result.waf_exclusion_rules
        ]

    if result.service_policies:
        lb_spec["active_service_policies"] = {
            "policies": [{"name": p.name, "namespace": namespace} for p in result.service_policies]
        }

    return lb


# Public API


def render_json(
    result: XCTranslationResult,
    namespace: str = "default",
    lb_name: str = "translated-lb",
) -> dict[str, Any]:
    """Render an XC translation result as ves.io JSON API objects.

    Returns a dict with ``origin_pools``, ``service_policies``,
    ``http_loadbalancer``, and ``summary`` keys.
    """
    output: dict[str, Any] = {}

    if result.origin_pools:
        output["origin_pools"] = [_render_origin_pool(p, namespace) for p in result.origin_pools]

    if result.service_policies:
        output["service_policies"] = [
            _render_service_policy(p, namespace) for p in result.service_policies
        ]

    output["http_loadbalancer"] = _render_load_balancer(result, namespace, lb_name)

    # Summary
    output["summary"] = {
        "coverage_pct": result.coverage_pct,
        "translatable": result.translatable_count,
        "partial": result.partial_count,
        "untranslatable": result.untranslatable_count,
        "advisory": result.advisory_count,
        "items": [
            {
                "status": item.status.name.lower(),
                "kind": item.kind.name.lower(),
                "command": item.irule_command,
                "xc_description": item.xc_description,
                "note": item.note,
                "diagnostic_code": item.diagnostic_code,
            }
            for item in result.items
        ],
    }

    return output
