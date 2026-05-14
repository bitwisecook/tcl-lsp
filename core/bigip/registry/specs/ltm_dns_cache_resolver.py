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
            "ltm_dns_cache_resolver",
            module="ltm",
            object_types=("dns cache resolver",),
        ),
        header_types=(("ltm", "dns cache resolver"),),
        properties=(
            BigipPropertySpec(
                name="allowed-query-time",
                value_type="integer",
                default="200 milliseconds",
            ),
            BigipPropertySpec(
                name="answer-default-zones",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="forward-zones",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="nameservers",
                        value_type="list",
                        in_sections=("forward-zones",),
                        allow_none=True,
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="nameservers",
                value_type="list",
                in_sections=("forward-zones",),
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="local-zones",
                value_type="list",
                repeated=True,
                list_operators=frozenset(("add",)),
                default="empty",
            ),
            BigipPropertySpec(name="max-concurrent-queries", value_type="integer", default="1024"),
            BigipPropertySpec(name="max-concurrent-tcp", value_type="integer", default="20"),
            BigipPropertySpec(name="max-concurrent-udp", value_type="integer", default="8192"),
            BigipPropertySpec(name="msg-cache-size", value_type="integer", default="1048576"),
            BigipPropertySpec(
                name="nameserver-cache-count",
                value_type="integer",
                default="16536 entries",
            ),
            BigipPropertySpec(
                name="nameserver-min-rtt",
                value_type="integer",
                default="50 milliseconds",
            ),
            BigipPropertySpec(name="nameserver-ttl", value_type="integer", default="900 seconds"),
            BigipPropertySpec(name="outbound-msg-retry", value_type="integer", default="5"),
            BigipPropertySpec(
                name="prefetch",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="randomize-query-name-case",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="response-policy-zones",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify")),
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("response-policy-zones",),
                        enum_values=("nxdomain", "walled-garden"),
                    ),
                    BigipPropertySpec(
                        name="walled-garden",
                        value_type="unknown",
                        in_sections=("response-policy-zones",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("response-policy-zones",),
                enum_values=("nxdomain", "walled-garden"),
            ),
            BigipPropertySpec(
                name="walled-garden",
                value_type="unknown",
                in_sections=("response-policy-zones",),
            ),
            BigipPropertySpec(name="root-hints", value_type="list"),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                references=("net_route_domain",),
                default="the default route domain",
            ),
            BigipPropertySpec(name="rrset-cache-size", value_type="integer", default="10485760"),
            BigipPropertySpec(
                name="rrset-rotate",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "query-id"),
                default="none",
            ),
            BigipPropertySpec(
                name="unwanted-query-reply-threshold",
                value_type="integer",
                default="0 (off)",
            ),
            BigipPropertySpec(
                name="use-ipv4",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="use-ipv6",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="use-tcp",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="use-udp",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
        ),
    )
