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
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="max-flowspec-routes-limit", value_type="integer"),
            BigipPropertySpec(
                name="neighbor",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
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
                name="graceful-restart-time",
                value_type="integer",
                in_sections=("neighbor",),
            ),
            BigipPropertySpec(name="hold-time", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(name="local-address", value_type="string", in_sections=("neighbor",)),
            BigipPropertySpec(name="local-as", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(name="remote-as", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(name="router-id", value_type="string", in_sections=("neighbor",)),
            BigipPropertySpec(name="peer-group", value_type="unknown"),
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
                name="graceful-restart-time",
                value_type="integer",
                in_sections=("peer-group",),
            ),
            BigipPropertySpec(name="hold-time", value_type="integer", in_sections=("peer-group",)),
            BigipPropertySpec(
                name="local-address",
                value_type="string",
                in_sections=("peer-group",),
            ),
            BigipPropertySpec(name="local-as", value_type="integer", in_sections=("peer-group",)),
            BigipPropertySpec(name="remote-as", value_type="integer", in_sections=("peer-group",)),
            BigipPropertySpec(name="router-id", value_type="string", in_sections=("peer-group",)),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="rules",
                value_type="list",
                allow_none=True,
                references=(
                    "gtm_rule",
                    "ltm_cipher_rule",
                    "ltm_global_settings_rule",
                    "ltm_rule",
                    "ltm_rule_profiler",
                    "security_firewall_matching_rule",
                    "security_firewall_on_demand_rule_deploy",
                    "security_firewall_rule_list",
                    "security_firewall_rule_stat",
                    "security_packet_filter_rule_stat",
                    "sys_file_rewrite_rule",
                ),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="action", value_type="unknown", in_sections=("rules",)),
            BigipPropertySpec(
                name="asn-community",
                value_type="string",
                in_sections=("rules", "action"),
            ),
            BigipPropertySpec(
                name="dscp-value",
                value_type="integer",
                in_sections=("rules", "action"),
            ),
            BigipPropertySpec(
                name="next-hop",
                value_type="string",
                in_sections=("rules", "action"),
            ),
            BigipPropertySpec(
                name="rate-limit",
                value_type="integer",
                in_sections=("rules", "action"),
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                in_sections=("rules", "action"),
                enum_values=("drop", "qos", "rate-limit", "redirect"),
            ),
            BigipPropertySpec(
                name="advertisement-ttl-from-now",
                value_type="integer",
                in_sections=("rules",),
            ),
            BigipPropertySpec(name="alias", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(name="app-service", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(name="description", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(name="match", value_type="unknown", in_sections=("rules",)),
            BigipPropertySpec(
                name="destination-address",
                value_type="string",
                in_sections=("rules", "match"),
            ),
            BigipPropertySpec(
                name="destination-ports",
                value_type="unknown",
                in_sections=("rules", "match"),
            ),
            BigipPropertySpec(
                name="dscp-values",
                value_type="integer",
                in_sections=("rules", "match"),
            ),
            BigipPropertySpec(
                name="icmp-codes",
                value_type="integer",
                in_sections=("rules", "match"),
            ),
            BigipPropertySpec(
                name="icmp-types",
                value_type="integer",
                in_sections=("rules", "match"),
            ),
            BigipPropertySpec(
                name="ip-fragments",
                value_type="integer",
                in_sections=("rules", "match"),
            ),
            BigipPropertySpec(
                name="ip-protocols",
                value_type="unknown",
                in_sections=("rules", "match"),
            ),
            BigipPropertySpec(
                name="packet-lengths",
                value_type="integer",
                in_sections=("rules", "match"),
            ),
            BigipPropertySpec(name="ports", value_type="unknown", in_sections=("rules", "match")),
            BigipPropertySpec(
                name="source-address",
                value_type="string",
                in_sections=("rules", "match"),
            ),
            BigipPropertySpec(
                name="source-ports",
                value_type="unknown",
                in_sections=("rules", "match"),
            ),
            BigipPropertySpec(name="tcp-flags", value_type="list", in_sections=("rules", "match")),
            BigipPropertySpec(
                name="bitwise-and-fields",
                value_type="integer",
                in_sections=("rules", "match", "tcp-flags"),
            ),
            BigipPropertySpec(
                name="bitwise-or-fields",
                value_type="integer",
                in_sections=("rules", "match", "tcp-flags"),
            ),
            BigipPropertySpec(
                name="remove-config-upon-expiry",
                value_type="unknown",
                in_sections=("rules",),
            ),
            BigipPropertySpec(name="security-log-profile", value_type="string"),
        ),
    )
