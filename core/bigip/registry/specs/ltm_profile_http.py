from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "ltm_profile_http",
            module="ltm",
            object_types=("profile http",),
        ),
        header_types=(("ltm", "profile http"),),
        properties=(
            BigipPropertySpec(
                name="accept-xff", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="basic-auth-realm", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_http",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="encrypt-cookie-secret",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "passphrase"),
            ),
            BigipPropertySpec(name="encrypt-cookies", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="enforcement", value_type="string"),
            BigipPropertySpec(
                name="rfc-compliance",
                value_type="enum",
                in_sections=("enforcement",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="allow-ws-header-name",
                value_type="enum",
                in_sections=("enforcement",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="excess-client-headers",
                value_type="enum",
                in_sections=("enforcement",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="excess-server-headers",
                value_type="enum",
                in_sections=("enforcement",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="max-header-size", value_type="integer", in_sections=("enforcement",)
            ),
            BigipPropertySpec(
                name="max-header-count", value_type="integer", in_sections=("enforcement",)
            ),
            BigipPropertySpec(
                name="max-requests", value_type="integer", in_sections=("enforcement",)
            ),
            BigipPropertySpec(
                name="oversize-client-headers",
                value_type="enum",
                in_sections=("enforcement",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="oversize-server-headers",
                value_type="enum",
                in_sections=("enforcement",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="pipeline",
                value_type="enum",
                in_sections=("enforcement",),
                enum_values=("allow", "pass-through", "reject"),
            ),
            BigipPropertySpec(
                name="truncated-redirects",
                value_type="enum",
                in_sections=("enforcement",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="unknown-method",
                value_type="enum",
                in_sections=("enforcement",),
                enum_values=("allow", "pass-through", "reject"),
            ),
            BigipPropertySpec(name="fallback-host", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="fallback-status-codes", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="header-erase", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="header-insert", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="insert-xforwarded-for", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="lws-separator", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="lws-width", value_type="integer"),
            BigipPropertySpec(
                name="oneconnect-transformations",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="oneconnect-status-reuse", value_type="string"),
            BigipPropertySpec(
                name="proxy-type",
                value_type="enum",
                enum_values=("reverse", "explicit", "transparent"),
            ),
            BigipPropertySpec(
                name="redirect-rewrite",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "matching", "nodes", "none"),
            ),
            BigipPropertySpec(
                name="request-chunking", value_type="enum", enum_values=("rechunk", "sustain")
            ),
            BigipPropertySpec(
                name="response-chunking",
                value_type="enum",
                enum_values=("rechunk", "sustain", "unchunk"),
            ),
            BigipPropertySpec(
                name="response-headers-permitted", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(name="server-agent-name", value_type="string"),
            BigipPropertySpec(name="explicit-proxy", value_type="string"),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("explicit-proxy",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="dns-resolver", value_type="string", in_sections=("explicit-proxy",)
            ),
            BigipPropertySpec(
                name="ipv6",
                value_type="enum",
                in_sections=("explicit-proxy",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="tunnel-name", value_type="string", in_sections=("explicit-proxy",)
            ),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                in_sections=("explicit-proxy",),
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="default-connect-handling",
                value_type="enum",
                in_sections=("explicit-proxy",),
                enum_values=("deny", "allow"),
            ),
            BigipPropertySpec(
                name="tunnel-on-any-request",
                value_type="enum",
                in_sections=("explicit-proxy",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="connect-error-message", value_type="string", in_sections=("explicit-proxy",)
            ),
            BigipPropertySpec(
                name="dns-error-message", value_type="string", in_sections=("explicit-proxy",)
            ),
            BigipPropertySpec(
                name="bad-request-message", value_type="string", in_sections=("explicit-proxy",)
            ),
            BigipPropertySpec(
                name="bad-response-message", value_type="string", in_sections=("explicit-proxy",)
            ),
            BigipPropertySpec(name="sflow", value_type="string"),
            BigipPropertySpec(name="poll-interval", value_type="integer", in_sections=("sflow",)),
            BigipPropertySpec(
                name="poll-interval-global",
                value_type="enum",
                in_sections=("sflow",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(name="sampling-rate", value_type="integer", in_sections=("sflow",)),
            BigipPropertySpec(
                name="sampling-rate-global",
                value_type="enum",
                in_sections=("sflow",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(name="via-host-name", value_type="string"),
            BigipPropertySpec(
                name="via-request", value_type="enum", enum_values=("append", "preserve", "remove")
            ),
            BigipPropertySpec(
                name="via-response", value_type="enum", enum_values=("append", "preserve", "remove")
            ),
            BigipPropertySpec(name="hsts", value_type="string"),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                in_sections=("hsts",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="maximum-age", value_type="integer", in_sections=("hsts",)),
            BigipPropertySpec(
                name="include-subdomains",
                value_type="enum",
                in_sections=("hsts",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="preload",
                value_type="enum",
                in_sections=("hsts",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
