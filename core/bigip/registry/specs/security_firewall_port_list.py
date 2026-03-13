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
            "security_firewall_port_list",
            module="security",
            object_types=("firewall port-list",),
        ),
        header_types=(("security", "firewall port-list"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ports",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="security", value_type="string"),
            BigipPropertySpec(name="ports", value_type="string", in_sections=("security",)),
            BigipPropertySpec(
                name="domain", value_type="list", in_sections=("ports",), repeated=True
            ),
            BigipPropertySpec(
                name="f5-iquery", value_type="list", in_sections=("ports",), repeated=True
            ),
            BigipPropertySpec(
                name="https", value_type="list", in_sections=("ports",), repeated=True
            ),
            BigipPropertySpec(
                name="snmp", value_type="list", in_sections=("ports",), repeated=True
            ),
            BigipPropertySpec(name="ssh", value_type="list", in_sections=("ports",), repeated=True),
            BigipPropertySpec(
                name="cap", value_type="list", in_sections=("security",), repeated=True
            ),
            BigipPropertySpec(
                name="domain", value_type="list", in_sections=("security",), repeated=True
            ),
            BigipPropertySpec(
                name="f5-iquery", value_type="list", in_sections=("security",), repeated=True
            ),
            BigipPropertySpec(
                name="snmp", value_type="list", in_sections=("security",), repeated=True
            ),
            BigipPropertySpec(
                name="http", value_type="list", in_sections=("ports",), repeated=True
            ),
        ),
    )
