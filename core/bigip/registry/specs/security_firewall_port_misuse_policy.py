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
            "security_firewall_port_misuse_policy",
            module="security",
            object_types=("firewall port-misuse-policy",),
        ),
        header_types=(("security", "firewall port-misuse-policy"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="drop-on-l7-mismatch", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="log-on-l7-mismatch", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="rules",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="drop-on-l7-mismatch",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("no", "yes", "use-policy-setting"),
            ),
            BigipPropertySpec(
                name="ip-protocol",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("sctp", "tcp", "udp"),
            ),
            BigipPropertySpec(name="l7-protocol", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="log-on-l7-mismatch",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("no", "yes", "use-policy-setting"),
            ),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("rules",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(name="security", value_type="string"),
            BigipPropertySpec(
                name="drop-on-l7-mismatch", value_type="boolean", in_sections=("security",)
            ),
            BigipPropertySpec(
                name="log-on-l7-mismatch", value_type="boolean", in_sections=("security",)
            ),
            BigipPropertySpec(name="rules", value_type="string", in_sections=("security",)),
            BigipPropertySpec(name="p80", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="drop-on-l7-mismatch", value_type="boolean", in_sections=("p80",)
            ),
            BigipPropertySpec(name="l7-protocol", value_type="string", in_sections=("p80",)),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("p80",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(name="p8080", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(name="l7-protocol", value_type="string", in_sections=("p8080",)),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("p8080",),
                min_value=0,
                max_value=65535,
            ),
        ),
    )
