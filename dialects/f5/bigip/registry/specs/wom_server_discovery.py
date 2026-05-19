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
            "wom_server_discovery",
            module="wom",
            object_types=("server-discovery",),
        ),
        header_types=(("wom", "server-discovery"),),
        properties=(
            BigipPropertySpec(
                name="auto-save",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="filter-mode",
                value_type="enum",
                enum_values=("exclude", "include"),
                default="exclude with no IP addresses specified, which means that all advertised routes that conform to the specified attributes are discovered",
            ),
            BigipPropertySpec(name="idle-time-limit", value_type="integer", default="0"),
            BigipPropertySpec(name="ip-ttl-limit", value_type="integer", default="5"),
            BigipPropertySpec(name="max-server-count", value_type="integer", default="50"),
            BigipPropertySpec(
                name="min-idle-time",
                value_type="integer",
                default="0, which indicates that idle time is not considered in discovery",
            ),
            BigipPropertySpec(name="min-prefix-length-ipv4", value_type="integer", default="32"),
            BigipPropertySpec(name="min-prefix-length-ipv6", value_type="integer", default="128"),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="rtt-threshold", value_type="integer", default="10"),
            BigipPropertySpec(
                name="subnet-filter",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                default="none",
            ),
            BigipPropertySpec(
                name="time-unit",
                value_type="enum",
                enum_values=("days", "hours", "minutes"),
            ),
        ),
    )
