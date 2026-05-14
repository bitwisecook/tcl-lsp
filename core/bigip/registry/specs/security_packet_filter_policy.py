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
            "security_packet_filter_policy",
            module="security",
            object_types=("packet-filter policy",),
        ),
        header_types=(("security", "packet-filter policy"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
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
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("rules",),
                        enum_values=("accept", "drop"),
                    ),
                    BigipPropertySpec(
                        name="description", value_type="string", in_sections=("rules",)
                    ),
                    BigipPropertySpec(
                        name="ipv6-extension-headers",
                        value_type="list",
                        in_sections=("rules",),
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="log",
                        value_type="enum",
                        in_sections=("rules",),
                        enum_values=("no", "yes"),
                        shape_kind="boolean",
                    ),
                    BigipPropertySpec(
                        name="status",
                        value_type="enum",
                        in_sections=("rules",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("accept", "drop"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="ipv6-extension-headers",
                value_type="list",
                in_sections=("rules",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="values",
                        value_type="list",
                        in_sections=("rules", "ipv6-extension-headers"),
                        required=True,
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="values",
                value_type="list",
                in_sections=("rules", "ipv6-extension-headers"),
                required=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="log",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="status",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
        ),
    )
