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
            "net_routing_route_map",
            module="net",
            object_types=("routing route-map",),
        ),
        header_types=(("net", "routing route-map"),),
        properties=(
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                allow_none=True,
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="entries",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="action", value_type="boolean", in_sections=("entries",), allow_none=True
            ),
            BigipPropertySpec(name="match", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(
                name="as-path", value_type="boolean", in_sections=("match",), allow_none=True
            ),
            BigipPropertySpec(name="community", value_type="string", in_sections=("match",)),
            BigipPropertySpec(
                name="exact-match",
                value_type="boolean",
                in_sections=("community",),
                allow_none=True,
            ),
            BigipPropertySpec(name="extcommunity", value_type="string", in_sections=("match",)),
            BigipPropertySpec(
                name="exact-match",
                value_type="boolean",
                in_sections=("extcommunity",),
                allow_none=True,
            ),
            BigipPropertySpec(name="ipv4", value_type="string", in_sections=("match",)),
            BigipPropertySpec(
                name="address",
                value_type="string",
                in_sections=("ipv4",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="access-list", value_type="boolean", in_sections=("address",), allow_none=True
            ),
            BigipPropertySpec(
                name="prefix-list", value_type="boolean", in_sections=("address",), allow_none=True
            ),
            BigipPropertySpec(name="next-hop", value_type="string", in_sections=("ipv4",)),
            BigipPropertySpec(
                name="access-list", value_type="boolean", in_sections=("next-hop",), allow_none=True
            ),
            BigipPropertySpec(
                name="prefix-list", value_type="boolean", in_sections=("next-hop",), allow_none=True
            ),
            BigipPropertySpec(name="peer", value_type="string", in_sections=("ipv4",)),
            BigipPropertySpec(
                name="access-list", value_type="boolean", in_sections=("peer",), allow_none=True
            ),
            BigipPropertySpec(name="ipv6", value_type="string", in_sections=("match",)),
            BigipPropertySpec(
                name="address",
                value_type="string",
                in_sections=("ipv6",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="next-hop", value_type="string", in_sections=("ipv6",)),
            BigipPropertySpec(name="peer", value_type="string", in_sections=("ipv6",)),
            BigipPropertySpec(name="metric", value_type="integer", in_sections=("match",)),
            BigipPropertySpec(
                name="origin", value_type="boolean", in_sections=("match",), allow_none=True
            ),
            BigipPropertySpec(
                name="route-type", value_type="boolean", in_sections=("match",), allow_none=True
            ),
            BigipPropertySpec(name="tag", value_type="integer", in_sections=("match",)),
            BigipPropertySpec(
                name="vlan",
                value_type="reference",
                in_sections=("match",),
                allow_none=True,
                references=("net_vlan",),
            ),
            BigipPropertySpec(name="set", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(name="aggregator", value_type="string", in_sections=("set",)),
            BigipPropertySpec(
                name="address",
                value_type="string",
                in_sections=("aggregator",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="as", value_type="integer", in_sections=("aggregator",)),
            BigipPropertySpec(name="as-path-prepend", value_type="integer", in_sections=("set",)),
            BigipPropertySpec(
                name="atomic-aggregate", value_type="boolean", in_sections=("set",), allow_none=True
            ),
            BigipPropertySpec(name="community", value_type="string", in_sections=("set",)),
            BigipPropertySpec(
                name="additive", value_type="boolean", in_sections=("community",), allow_none=True
            ),
            BigipPropertySpec(
                name="exact-set", value_type="boolean", in_sections=("community",), allow_none=True
            ),
            BigipPropertySpec(name="value", value_type="integer", in_sections=("community",)),
            BigipPropertySpec(name="dampening", value_type="string", in_sections=("set",)),
            BigipPropertySpec(
                name="reachability-half-life", value_type="integer", in_sections=("dampening",)
            ),
            BigipPropertySpec(name="reuse", value_type="integer", in_sections=("dampening",)),
            BigipPropertySpec(name="suppress", value_type="integer", in_sections=("dampening",)),
            BigipPropertySpec(
                name="suppress-max", value_type="integer", in_sections=("dampening",)
            ),
            BigipPropertySpec(
                name="unreachability-half-life", value_type="integer", in_sections=("dampening",)
            ),
            BigipPropertySpec(name="extcommunity", value_type="string", in_sections=("set",)),
            BigipPropertySpec(
                name="rt", value_type="boolean", in_sections=("extcommunity",), allow_none=True
            ),
            BigipPropertySpec(
                name="soo", value_type="boolean", in_sections=("extcommunity",), allow_none=True
            ),
            BigipPropertySpec(name="ip", value_type="string", in_sections=("set",)),
            BigipPropertySpec(name="next-hop", value_type="string", in_sections=("ip",)),
            BigipPropertySpec(
                name="address",
                value_type="string",
                in_sections=("next-hop",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="ipv6", value_type="string", in_sections=("set",)),
            BigipPropertySpec(name="local", value_type="string", in_sections=("next-hop",)),
            BigipPropertySpec(
                name="level", value_type="boolean", in_sections=("set",), allow_none=True
            ),
            BigipPropertySpec(name="local-preference", value_type="integer", in_sections=("set",)),
            BigipPropertySpec(name="metric", value_type="string", in_sections=("set",)),
            BigipPropertySpec(
                name="type", value_type="boolean", in_sections=("metric",), allow_none=True
            ),
            BigipPropertySpec(
                name="value", value_type="boolean", in_sections=("metric",), allow_none=True
            ),
            BigipPropertySpec(
                name="origin", value_type="boolean", in_sections=("set",), allow_none=True
            ),
            BigipPropertySpec(name="originator-id", value_type="integer", in_sections=("set",)),
            BigipPropertySpec(name="tag", value_type="integer", in_sections=("set",)),
            BigipPropertySpec(name="weight", value_type="integer", in_sections=("set",)),
        ),
    )
