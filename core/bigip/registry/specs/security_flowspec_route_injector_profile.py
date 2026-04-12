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
            "security_flowspec_route_injector_profile",
            module="security",
            object_types=("flowspec-route-injector profile",),
        ),
        header_types=(("security", "flowspec-route-injector profile"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="max-flowspec-routes-limit", value_type="integer"),
            BigipPropertySpec(
                name="neighbor",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="adj-out",
                value_type="enum",
                in_sections=("neighbor",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="bgp-multiple-instance",
                value_type="enum",
                in_sections=("neighbor",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="extended-asn-cap",
                value_type="enum",
                in_sections=("neighbor",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="graceful-restart",
                value_type="enum",
                in_sections=("neighbor",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="graceful-restart-time", value_type="integer", in_sections=("neighbor",)
            ),
            BigipPropertySpec(name="hold-time", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(
                name="local-address",
                value_type="string",
                in_sections=("neighbor",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="local-as", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(name="remote-as", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(name="router-id", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(
                name="rules",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="action", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(name="dscp-value", value_type="integer", in_sections=("action",)),
            BigipPropertySpec(name="next-hop", value_type="string", in_sections=("action",)),
            BigipPropertySpec(name="rate-limit", value_type="integer", in_sections=("action",)),
            BigipPropertySpec(name="asn-community", value_type="string", in_sections=("action",)),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                in_sections=("action",),
                enum_values=("drop", "redirect", "rate-limit", "qos"),
            ),
            BigipPropertySpec(name="alias", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="advertisement-ttl-from-now", value_type="integer", in_sections=("rules",)
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="remove-config-upon-expiry", value_type="string", in_sections=("rules",)
            ),
            BigipPropertySpec(name="match", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="destination-address",
                value_type="string",
                in_sections=("match",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="destination-ports", value_type="string", in_sections=("match",)
            ),
            BigipPropertySpec(name="dscp-values", value_type="integer", in_sections=("match",)),
            BigipPropertySpec(name="icmp-codes", value_type="integer", in_sections=("match",)),
            BigipPropertySpec(name="icmp-types", value_type="integer", in_sections=("match",)),
            BigipPropertySpec(name="ip-fragments", value_type="integer", in_sections=("match",)),
            BigipPropertySpec(name="ip-protocols", value_type="string", in_sections=("match",)),
            BigipPropertySpec(name="packet-lengths", value_type="integer", in_sections=("match",)),
            BigipPropertySpec(name="ports", value_type="string", in_sections=("match",)),
            BigipPropertySpec(
                name="source-address",
                value_type="string",
                in_sections=("match",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="source-ports", value_type="string", in_sections=("match",)),
            BigipPropertySpec(name="tcp-flags", value_type="string", in_sections=("match",)),
            BigipPropertySpec(
                name="bitwise-and-fields", value_type="integer", in_sections=("tcp-flags",)
            ),
            BigipPropertySpec(
                name="bitwise-or-fields", value_type="integer", in_sections=("tcp-flags",)
            ),
            BigipPropertySpec(
                name="route-domain", value_type="reference", references=("net_route_domain",)
            ),
            BigipPropertySpec(name="peer-group", value_type="string"),
            BigipPropertySpec(
                name="adj-out",
                value_type="enum",
                in_sections=("peer-group",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="bgp-multiple-instance",
                value_type="enum",
                in_sections=("peer-group",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="extended-asn-cap",
                value_type="enum",
                in_sections=("peer-group",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="graceful-restart",
                value_type="enum",
                in_sections=("peer-group",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="graceful-restart-time", value_type="integer", in_sections=("peer-group",)
            ),
            BigipPropertySpec(name="hold-time", value_type="integer", in_sections=("peer-group",)),
            BigipPropertySpec(
                name="local-address",
                value_type="string",
                in_sections=("peer-group",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="local-as", value_type="integer", in_sections=("peer-group",)),
            BigipPropertySpec(name="remote-as", value_type="integer", in_sections=("peer-group",)),
            BigipPropertySpec(name="router-id", value_type="integer", in_sections=("peer-group",)),
            BigipPropertySpec(name="security-log-profile", value_type="string"),
        ),
    )
