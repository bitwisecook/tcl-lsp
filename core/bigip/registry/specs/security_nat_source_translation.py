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
            "security_nat_source_translation",
            module="security",
            object_types=("nat source-translation",),
        ),
        header_types=(("security", "nat source-translation"),),
        properties=(
            BigipPropertySpec(
                name="addresses",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="client-connection-limit", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="exclude-addresses",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="exclude-address-lists",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "default", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="hairpin-mode", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="icmp-echo", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="inbound-mode",
                value_type="enum",
                allow_none=True,
                enum_values=("endpoint-independent-filtering", "explicit", "none"),
            ),
            BigipPropertySpec(name="eif-timeout", value_type="integer"),
            BigipPropertySpec(
                name="pat-mode", value_type="enum", enum_values=("deterministic", "napt", "pba")
            ),
            BigipPropertySpec(name="pcp", value_type="string"),
            BigipPropertySpec(
                name="profile", value_type="boolean", in_sections=("pcp",), allow_none=True
            ),
            BigipPropertySpec(
                name="selfip", value_type="boolean", in_sections=("pcp",), allow_none=True
            ),
            BigipPropertySpec(
                name="ports",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="proxy-arp", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="route-advertisement", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=("dynamic-pat", "static-nat", "static-pat"),
            ),
            BigipPropertySpec(name="mapping", value_type="string"),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                in_sections=("mapping",),
                allow_none=True,
                enum_values=("address-pooling-paired", "endpoint-independent-mapping", "none"),
            ),
            BigipPropertySpec(name="timeout", value_type="integer", in_sections=("mapping",)),
            BigipPropertySpec(name="port-block-allocation", value_type="string"),
            BigipPropertySpec(
                name="block-idle-timeout",
                value_type="integer",
                in_sections=("port-block-allocation",),
            ),
            BigipPropertySpec(
                name="block-lifetime", value_type="integer", in_sections=("port-block-allocation",)
            ),
            BigipPropertySpec(
                name="block-size", value_type="integer", in_sections=("port-block-allocation",)
            ),
            BigipPropertySpec(
                name="client-block-limit",
                value_type="integer",
                in_sections=("port-block-allocation",),
            ),
            BigipPropertySpec(
                name="zombie-timeout", value_type="integer", in_sections=("port-block-allocation",)
            ),
        ),
    )
