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
            "ltm_global_settings_traffic_control",
            module="ltm",
            object_types=("global-settings traffic-control",),
        ),
        header_types=(("ltm", "global-settings traffic-control"),),
        properties=(
            BigipPropertySpec(
                name="accept-ip-options", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="accept-ip-source-route",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="allow-ip-source-route", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="continue-matching", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="max-icmp-rate", value_type="integer"),
            BigipPropertySpec(name="max-reject-rate", value_type="integer"),
            BigipPropertySpec(name="max-reject-rate-timeout", value_type="integer"),
            BigipPropertySpec(name="min-path-mtu", value_type="integer"),
            BigipPropertySpec(
                name="path-mtu-discovery", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="port-find-linear", value_type="integer"),
            BigipPropertySpec(name="port-find-random", value_type="integer"),
            BigipPropertySpec(
                name="port-find-threshold-warning",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="port-find-threshold-trigger", value_type="integer"),
            BigipPropertySpec(name="port-find-threshold-timeout", value_type="integer"),
            BigipPropertySpec(
                name="reject-unmatched", value_type="enum", enum_values=("disabled", "enabled")
            ),
        ),
    )
