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
            "security_firewall_rule_list",
            module="security",
            object_types=("firewall rule-list",),
        ),
        header_types=(("security", "firewall rule-list"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="rules",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("accept", "accept-decisively", "drop", "reject"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(name="source", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="address-lists",
                value_type="enum",
                in_sections=("source",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="addresses",
                value_type="enum",
                in_sections=("source",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="fqdns",
                value_type="enum",
                in_sections=("source",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="geo",
                value_type="enum",
                in_sections=("source",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="ipi-category",
                value_type="enum",
                in_sections=("source",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="port-lists",
                value_type="enum",
                in_sections=("source",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="ports",
                value_type="enum",
                in_sections=("source",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="vlans",
                value_type="reference",
                in_sections=("source",),
                enum_values=("add", "default", "delete", "replace-all-with"),
                references=("net_vlan",),
            ),
            BigipPropertySpec(name="destination", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="address-lists",
                value_type="enum",
                in_sections=("destination",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="addresses",
                value_type="enum",
                in_sections=("destination",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="fqdns",
                value_type="enum",
                in_sections=("destination",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="geo",
                value_type="enum",
                in_sections=("destination",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="ipi-category",
                value_type="enum",
                in_sections=("destination",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="port-lists",
                value_type="enum",
                in_sections=("destination",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="ports",
                value_type="enum",
                in_sections=("destination",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="icmp",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("icmp",)),
            BigipPropertySpec(name="icmp", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="ip-protocol", value_type="string"),
            BigipPropertySpec(name="irule", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(name="irule-sample-rate", value_type="integer"),
            BigipPropertySpec(name="log", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="place-after", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(
                name="place-before", value_type="reference", references=("ltm_rule",)
            ),
            BigipPropertySpec(name="rule-list", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(name="schedule", value_type="string"),
            BigipPropertySpec(
                name="status", value_type="enum", enum_values=("disabled", "enabled", "scheduled")
            ),
            BigipPropertySpec(
                name="service-policy",
                value_type="reference",
                references=("security_firewall_policy", "ltm_policy"),
            ),
            BigipPropertySpec(name="uuid", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="virtual-server", value_type="string"),
            BigipPropertySpec(name="ips-profile", value_type="string"),
            BigipPropertySpec(name="classification-policy", value_type="string"),
            BigipPropertySpec(name="security", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(name="description", value_type="string", in_sections=("security",)),
            BigipPropertySpec(name="rules", value_type="string", in_sections=("security",)),
            BigipPropertySpec(
                name="telnet", value_type="list", in_sections=("ports",), repeated=True
            ),
            BigipPropertySpec(name="ip-protocol", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(name="action", value_type="string", in_sections=("security",)),
            BigipPropertySpec(name="destination", value_type="string", in_sections=("security",)),
            BigipPropertySpec(
                name="http", value_type="list", in_sections=("ports",), repeated=True
            ),
            BigipPropertySpec(name="ip-protocol", value_type="string", in_sections=("security",)),
            BigipPropertySpec(name="r2", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(name="action", value_type="string", in_sections=("r2",)),
            BigipPropertySpec(name="source", value_type="string", in_sections=("r2",)),
            BigipPropertySpec(
                name="state", value_type="boolean", in_sections=("geo",), allow_none=True
            ),
            BigipPropertySpec(name="r1", value_type="string", in_sections=("security",)),
            BigipPropertySpec(name="action", value_type="string", in_sections=("r1",)),
            BigipPropertySpec(name="source", value_type="string", in_sections=("r1",)),
            BigipPropertySpec(name="r1", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(name="destination", value_type="string", in_sections=("r1",)),
        ),
    )
