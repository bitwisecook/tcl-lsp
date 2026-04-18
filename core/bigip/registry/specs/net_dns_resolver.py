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
                name="answer-default-zones", value_type="enum", enum_values=("yes", "no")
            ),
            BigipPropertySpec(name="cache-size", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
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
            BigipPropertySpec(name="nameserver-ttl", value_type="integer"),
            BigipPropertySpec(name="nameserver-min-rtt", value_type="integer"),
            BigipPropertySpec(
                name="randomize-query-name-case", value_type="enum", enum_values=("yes", "no")
            ),
            BigipPropertySpec(name="prefetch", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="outbound-msg-retry", value_type="integer"),
            BigipPropertySpec(
                name="route-domain", value_type="reference", references=("net_route_domain",)
            ),
            BigipPropertySpec(name="use-ipv4", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="use-ipv6", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="use-tcp", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="use-udp", value_type="enum", enum_values=("yes", "no")),
        ),
    )
