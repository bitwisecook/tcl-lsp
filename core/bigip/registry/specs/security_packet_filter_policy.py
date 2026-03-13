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
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
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
                value_type="enum",
                in_sections=("rules",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="values",
                value_type="enum",
                in_sections=("ipv6-extension-headers",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="log", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="status", value_type="enum", enum_values=("disabled", "enabled")
            ),
        ),
    )
