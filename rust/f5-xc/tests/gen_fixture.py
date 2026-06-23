"""Generate the f5-xc parity fixture from the Python translator oracle.

Runs the real `dialects.f5.xc.translator.translate_irule` (+ diagnostics)
over a representative iRule corpus and writes a JSON fixture the Rust
`f5-xc` parity test asserts against.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT))

from compiler.registry.runtime import configure_signatures  # noqa: E402
from dialects.f5.xc.diagnostics import get_xc_diagnostics  # noqa: E402
from dialects.f5.xc.translator import translate_irule  # noqa: E402
from dialects.f5.xc.xc_model import TranslateStatus  # noqa: E402

CORPUS: list[tuple[str, str]] = [
    ("simple_pool", "when HTTP_REQUEST {\n    pool my_pool\n}"),
    (
        "dedup_pools",
        'when HTTP_REQUEST {\n    if {[HTTP::path] eq "/a"} {\n        pool shared_pool\n    } else {\n        pool shared_pool\n    }\n}',
    ),
    (
        "switch_path_glob",
        'when HTTP_REQUEST {\n    switch -glob [HTTP::path] {\n        "/api/*" { pool api_pool }\n        "/static/*" { pool static_pool }\n        default { pool default_pool }\n    }\n}',
    ),
    (
        "switch_host",
        'when HTTP_REQUEST {\n    switch [HTTP::host] {\n        "api.example.com" { pool api_pool }\n        "www.example.com" { pool web_pool }\n    }\n}',
    ),
    (
        "switch_method",
        'when HTTP_REQUEST {\n    switch [HTTP::method] {\n        "GET" { pool read_pool }\n        "POST" { pool write_pool }\n    }\n}',
    ),
    (
        "switch_dynamic",
        'when HTTP_REQUEST {\n    switch [HTTP::header value "X-Route"] {\n        "a" { pool a_pool }\n    }\n}',
    ),
    (
        "if_path_exact",
        'when HTTP_REQUEST {\n    if {[HTTP::path] eq "/login"} {\n        pool login_pool\n    }\n}',
    ),
    (
        "if_path_starts_with",
        'when HTTP_REQUEST {\n    if {[HTTP::path] starts_with "/api"} {\n        pool api_pool\n    }\n}',
    ),
    (
        "if_host_method_and",
        'when HTTP_REQUEST {\n    if {[HTTP::host] eq "api.example.com" && [HTTP::method] eq "POST"} {\n        pool api_pool\n    }\n}',
    ),
    (
        "if_or_branches",
        'when HTTP_REQUEST {\n    if {[HTTP::path] eq "/a" || [HTTP::path] eq "/b"} {\n        pool ab_pool\n    }\n}',
    ),
    (
        "if_header_match",
        'when HTTP_REQUEST {\n    if {[HTTP::header value "X-Foo"] eq "bar"} {\n        pool foo_pool\n    }\n}',
    ),
    (
        "if_header_exists",
        'when HTTP_REQUEST {\n    if {[HTTP::header exists "X-Api-Key"]} {\n        pool api_pool\n    }\n}',
    ),
    (
        "if_cookie_match",
        'when HTTP_REQUEST {\n    if {[HTTP::cookie "session"] eq "abc"} {\n        pool sess_pool\n    }\n}',
    ),
    (
        "if_query_contains",
        'when HTTP_REQUEST {\n    if {[HTTP::query] contains "debug=1"} {\n        pool debug_pool\n    }\n}',
    ),
    (
        "if_negated_method",
        'when HTTP_REQUEST {\n    if {[HTTP::method] ne "GET"} {\n        pool nonget_pool\n    }\n}',
    ),
    (
        "if_unmappable",
        "when HTTP_REQUEST {\n    if {[clientside] && rand() > 0.5} {\n        pool x_pool\n    }\n}",
    ),
    (
        "redirect",
        'when HTTP_REQUEST {\n    if {[HTTP::path] eq "/old"} {\n        HTTP::redirect "https://new.example.com"\n    }\n}',
    ),
    (
        "respond_deny",
        'when HTTP_REQUEST {\n    if {[HTTP::path] starts_with "/admin"} {\n        HTTP::respond 403 content "Forbidden"\n    }\n}',
    ),
    (
        "respond_200",
        'when HTTP_REQUEST {\n    if {[HTTP::path] eq "/health"} {\n        HTTP::respond 200 content "OK"\n    }\n}',
    ),
    (
        "header_insert",
        'when HTTP_REQUEST {\n    HTTP::header insert "X-Custom" "value"\n}',
    ),
    (
        "header_replace_response",
        'when HTTP_RESPONSE {\n    HTTP::header replace "Server" "Acme"\n}',
    ),
    (
        "header_remove",
        'when HTTP_REQUEST {\n    HTTP::header remove "X-Internal"\n}',
    ),
    (
        "class_match",
        "when HTTP_REQUEST {\n    if {[class match [HTTP::path] equals url_allowlist]} {\n        pool allow_pool\n    }\n}",
    ),
    (
        "asm_disable",
        'when HTTP_REQUEST {\n    if {[HTTP::path] starts_with "/api"} {\n        ASM::disable\n    }\n}',
    ),
    ("asm_enable", "when HTTP_REQUEST {\n    ASM::enable\n}"),
    (
        "loop_foreach",
        "when HTTP_REQUEST {\n    foreach h [HTTP::header names] {\n        log local0. $h\n    }\n}",
    ),
    (
        "eval_barrier",
        "when HTTP_REQUEST {\n    eval $dynamic_code\n}",
    ),
    (
        "untranslatable_event",
        "when CLIENT_ACCEPTED {\n    set foo 1\n}",
    ),
    (
        "advisory_event",
        "when CLIENTSSL_HANDSHAKE {\n    log local0. tls\n}",
    ),
    (
        "unknown_event",
        "when MADE_UP_EVENT {\n    set x 1\n}",
    ),
    (
        "untranslatable_prefix_tcp",
        "when HTTP_REQUEST {\n    TCP::respond foo\n}",
    ),
    (
        "string_tolower_wrapper",
        'when HTTP_REQUEST {\n    if {[string tolower [HTTP::path]] eq "/api"} {\n        pool api_pool\n    }\n}',
    ),
    (
        "nested_switch_in_if",
        'when HTTP_REQUEST {\n    if {[HTTP::host] eq "api.example.com"} {\n        switch -glob [HTTP::path] {\n            "/v1/*" { pool v1_pool }\n            "/v2/*" { pool v2_pool }\n        }\n    }\n}',
    ),
    (
        "multi_event",
        'when HTTP_REQUEST {\n    pool web_pool\n}\nwhen HTTP_RESPONSE {\n    HTTP::header insert "X-Served-By" "xc"\n}',
    ),
    (
        "ends_with_path",
        'when HTTP_REQUEST {\n    if {[HTTP::path] ends_with ".jpg"} {\n        pool img_pool\n    }\n}',
    ),
    (
        "matches_glob_path",
        'when HTTP_REQUEST {\n    if {[HTTP::path] matches_glob "/api/*/v2"} {\n        pool g_pool\n    }\n}',
    ),
    (
        "empty_rule",
        "when HTTP_REQUEST {\n}",
    ),
    (
        "ip_class_match",
        "when HTTP_REQUEST {\n    if {[class match [IP::client_addr] equals trusted_ips]} {\n        ASM::disable\n    }\n}",
    ),
]


def summarise(source: str) -> dict:
    result = translate_irule(source)
    diags = get_xc_diagnostics(source)
    item_codes = sorted(i.diagnostic_code for i in result.items if i.diagnostic_code)
    return {
        "item_codes": item_codes,
        "statuses": {
            "translated": sum(1 for i in result.items if i.status == TranslateStatus.TRANSLATED),
            "partial": sum(1 for i in result.items if i.status == TranslateStatus.PARTIAL),
            "untranslatable": sum(
                1 for i in result.items if i.status == TranslateStatus.UNTRANSLATABLE
            ),
            "advisory": sum(1 for i in result.items if i.status == TranslateStatus.ADVISORY),
        },
        "coverage_pct": result.coverage_pct,
        "counts": {
            "routes": len(result.routes),
            "origin_pools": len(result.origin_pools),
            "service_policies": len(result.service_policies),
            "service_policy_rules": sum(len(p.rules) for p in result.service_policies),
            "header_actions": len(result.header_actions),
            "waf_exclusion_rules": len(result.waf_exclusion_rules),
        },
        "origin_pool_names": sorted(p.name for p in result.origin_pools),
        "diag_codes": sorted(d.code for d in diags),
        "diag_messages": sorted(d.message for d in diags),
    }


def main() -> None:
    configure_signatures(dialect="f5-irules")
    fixture = []
    for name, source in CORPUS:
        fixture.append({"name": name, "source": source, "expected": summarise(source)})
    out = Path(__file__).resolve().parent / "parity_fixture.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(fixture, indent=2, ensure_ascii=False) + "\n")
    print(f"wrote {out} ({len(fixture)} cases)")


if __name__ == "__main__":
    main()
