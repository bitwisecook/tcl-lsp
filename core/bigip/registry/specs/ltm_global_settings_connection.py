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
            "ltm_global_settings_connection",
            module="ltm",
            object_types=("global-settings connection",),
        ),
        header_types=(("ltm", "global-settings connection"),),
        properties=(
            BigipPropertySpec(
                name="adaptive-reaper-hiwater",
                value_type="integer",
                default="95",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="adaptive-reaper-lowater",
                value_type="integer",
                default="85",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="auto-last-hop",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="default-vs-syn-challenge-threshold",
                value_type="integer",
                enum_values=("infinite",),
                default="infinite",
            ),
            BigipPropertySpec(name="global-flow-eviction-policy", value_type="reference"),
            BigipPropertySpec(
                name="global-syn-challenge-threshold",
                value_type="integer",
                enum_values=("infinite",),
                default="64K",
            ),
            BigipPropertySpec(
                name="syncookies-threshold",
                value_type="integer",
                default="16384",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="vlan-keyed-conn",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="vlan-syn-cookie",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
        ),
    )
