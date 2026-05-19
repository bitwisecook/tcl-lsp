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
            "net_timer_policy",
            module="net",
            object_types=("timer-policy",),
        ),
        header_types=(("net", "timer-policy"),),
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
                        name="description", value_type="string", in_sections=("rules",)
                    ),
                    BigipPropertySpec(
                        name="destination-ports",
                        value_type="list",
                        in_sections=("rules",),
                        allow_none=True,
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="ip-protocol",
                        value_type="reference",
                        in_sections=("rules",),
                    ),
                    BigipPropertySpec(
                        name="timers",
                        value_type="list",
                        in_sections=("rules",),
                        allow_none=True,
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                ),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="destination-ports",
                value_type="list",
                in_sections=("rules",),
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="ip-protocol", value_type="reference", in_sections=("rules",)),
            BigipPropertySpec(
                name="timers",
                value_type="list",
                in_sections=("rules",),
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="value",
                        value_type="unknown",
                        in_sections=("rules", "timers"),
                    ),
                ),
            ),
            BigipPropertySpec(name="value", value_type="unknown", in_sections=("rules", "timers")),
        ),
    )
