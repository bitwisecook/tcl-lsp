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
            "net_routing_profile_bgp",
            module="net",
            object_types=("routing profile bgp",),
        ),
        header_types=(("net", "routing profile bgp"),),
        properties=(
            BigipPropertySpec(
                name="adj-out", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="aggregate-nexthop-check",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="as-local-count", value_type="integer"),
            BigipPropertySpec(
                name="bgp-multiple-instance", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="extended-asn-cap", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="max-paths", value_type="string"),
            BigipPropertySpec(name="ebgp", value_type="integer", in_sections=("max-paths",)),
            BigipPropertySpec(name="ibgp", value_type="integer", in_sections=("max-paths",)),
            BigipPropertySpec(name="nexthop-trigger", value_type="string"),
            BigipPropertySpec(name="delay", value_type="integer", in_sections=("nexthop-trigger",)),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                in_sections=("nexthop-trigger",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="rfc1771", value_type="string"),
            BigipPropertySpec(
                name="path-select",
                value_type="enum",
                in_sections=("rfc1771",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="strict",
                value_type="enum",
                in_sections=("rfc1771",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="router-id", value_type="integer"),
        ),
    )
