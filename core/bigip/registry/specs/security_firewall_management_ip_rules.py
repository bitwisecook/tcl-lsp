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
            "security_firewall_management_ip_rules",
            module="security",
            object_types=("firewall management-ip-rules",),
        ),
        header_types=(("security", "firewall management-ip-rules"),),
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
                allow_none=True,
                enum_values=("add", "default", "delete", "none", "replace-all-with"),
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
            BigipPropertySpec(name="log", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="place-after", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(
                name="place-before", value_type="reference", references=("ltm_rule",)
            ),
            BigipPropertySpec(name="rule-list", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(name="schedule", value_type="string"),
            BigipPropertySpec(name="source", value_type="string"),
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
            BigipPropertySpec(
                name="status", value_type="enum", enum_values=("disabled", "enabled", "scheduled")
            ),
            BigipPropertySpec(name="uuid", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="security", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(name="rules", value_type="string", in_sections=("security",)),
            BigipPropertySpec(
                name="reject-insecure-ports", value_type="string", in_sections=("rules",)
            ),
            BigipPropertySpec(
                name="rule-list", value_type="string", in_sections=("reject-insecure-ports",)
            ),
            BigipPropertySpec(
                name="reject-internal-net", value_type="string", in_sections=("rules",)
            ),
            BigipPropertySpec(
                name="action", value_type="string", in_sections=("reject-internal-net",)
            ),
            BigipPropertySpec(
                name="source", value_type="string", in_sections=("reject-internal-net",)
            ),
        ),
    )
