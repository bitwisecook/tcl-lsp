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
            "ltm_dns_cache_validating_resolver",
            module="ltm",
            object_types=("dns cache validating-resolver",),
        ),
        header_types=(("ltm", "dns cache validating-resolver"),),
        properties=(
            BigipPropertySpec(name="allowed-query-time", value_type="integer"),
            BigipPropertySpec(
                name="answer-default-zones", value_type="enum", enum_values=("yes", "no")
            ),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="dlv-anchors", value_type="string"),
            BigipPropertySpec(
                name="forward-zones",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="nameservers",
                value_type="enum",
                in_sections=("forward-zones",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="ignore-cd", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="key-cache-size", value_type="integer"),
            BigipPropertySpec(name="local-zones", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="max-concurrent-queries", value_type="integer"),
            BigipPropertySpec(name="max-concurrent-udp", value_type="integer"),
            BigipPropertySpec(name="max-concurrent-tcp", value_type="integer"),
            BigipPropertySpec(name="msg-cache-size", value_type="integer"),
            BigipPropertySpec(name="nameserver-cache-count", value_type="integer"),
            BigipPropertySpec(name="nameserver-ttl", value_type="integer"),
            BigipPropertySpec(name="prefetch", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="prefetch-key", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="nameserver-min-rtt", value_type="integer"),
            BigipPropertySpec(name="outbound-msg-retry", value_type="integer"),
            BigipPropertySpec(
                name="randomize-query-name-case", value_type="enum", enum_values=("yes", "no")
            ),
            BigipPropertySpec(
                name="response-policy-zones",
                value_type="enum",
                enum_values=("add", "delete", "modify"),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("response-policy-zones",),
                enum_values=("nxdomain", "walled-garden"),
            ),
            BigipPropertySpec(
                name="walled-garden", value_type="string", in_sections=("response-policy-zones",)
            ),
            BigipPropertySpec(name="root-hints", value_type="string"),
            BigipPropertySpec(
                name="route-domain", value_type="reference", references=("net_route_domain",)
            ),
            BigipPropertySpec(name="rrset-cache-size", value_type="integer"),
            BigipPropertySpec(
                name="rrset-rotate",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "query-id"),
            ),
            BigipPropertySpec(name="trust-anchors", value_type="string"),
            BigipPropertySpec(name="unwanted-query-reply-threshold", value_type="integer"),
            BigipPropertySpec(name="use-ipv4", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="use-ipv6", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="use-tcp", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="use-udp", value_type="enum", enum_values=("yes", "no")),
        ),
    )
