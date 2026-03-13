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
            "ltm_global_settings_general",
            module="ltm",
            object_types=("global-settings general",),
        ),
        header_types=(("ltm", "global-settings general"),),
        properties=(
            BigipPropertySpec(name="gratuitous-arp-rate", value_type="integer"),
            BigipPropertySpec(name="l2-cache-timeout", value_type="integer"),
            BigipPropertySpec(
                name="maintenance-mode", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="mgmt-auto-lasthop", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="share-single-mac",
                value_type="enum",
                enum_values=("unique", "global", "vmw-compat"),
            ),
            BigipPropertySpec(
                name="snat-packet-forward", value_type="enum", enum_values=("disabled", "enabled")
            ),
        ),
    )
