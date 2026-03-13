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
            "security_analytics_settings",
            module="security",
            object_types=("analytics settings",),
        ),
        header_types=(("security", "analytics settings"),),
        properties=(
            BigipPropertySpec(name="acl-rules", value_type="string"),
            BigipPropertySpec(
                name="collect-client-ip",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collect-client-port",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("enabled", "disabled"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="collect-dest-ip",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collect-dest-port",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("enabled", "disabled"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="collect-server-side-stats",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collected-stats-internal-logging",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collected-stats-external-logging",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="dns", value_type="string"),
            BigipPropertySpec(
                name="collect-client-ip",
                value_type="enum",
                in_sections=("dns",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="dos-l2-l4", value_type="string"),
            BigipPropertySpec(
                name="collect-client-ip",
                value_type="enum",
                in_sections=("dos-l2-l4",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="l3-l4-errors", value_type="string"),
            BigipPropertySpec(
                name="collect-client-ip",
                value_type="enum",
                in_sections=("l3-l4-errors",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collect-dest-ip",
                value_type="enum",
                in_sections=("l3-l4-errors",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="publisher", value_type="string"),
            BigipPropertySpec(name="smtp-config", value_type="string"),
            BigipPropertySpec(name="stale-rules", value_type="string"),
            BigipPropertySpec(
                name="collect",
                value_type="enum",
                in_sections=("stale-rules",),
                enum_values=("enabled", "disabled"),
            ),
        ),
    )
