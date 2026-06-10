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
            "net_dns_resolver",
            module="net",
            object_types=("dns-resolver",),
        ),
        header_types=(("net", "dns-resolver"),),
        properties=(
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
            BigipPropertySpec(name="cache-size", value_type="integer", default="5767168"),
            BigipPropertySpec(name="description", value_type="string"),
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
                name="nameserver-min-rtt",
                value_type="integer",
                default="50 milliseconds",
            ),
            BigipPropertySpec(name="nameserver-ttl", value_type="integer", default="900 seconds"),
            BigipPropertySpec(name="outbound-msg-retry", value_type="integer"),
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
                name="route-domain",
                value_type="reference",
                references=("net_route_domain",),
                default="the default route domain",
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
