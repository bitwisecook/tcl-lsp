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
            BigipPropertySpec(name="adaptive-reaper-hiwater", value_type="integer"),
            BigipPropertySpec(name="adaptive-reaper-lowater", value_type="integer"),
            BigipPropertySpec(
                name="auto-last-hop", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="default-vs-syn-challenge-threshold", value_type="integer"),
            BigipPropertySpec(name="global-flow-eviction-policy", value_type="string"),
            BigipPropertySpec(name="global-syn-challenge-threshold", value_type="integer"),
            BigipPropertySpec(name="syncookies-threshold", value_type="integer"),
            BigipPropertySpec(
                name="vlan-keyed-conn", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="vlan-syn-cookie", value_type="enum", enum_values=("disabled", "enabled")
            ),
        ),
    )
