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
            "pem_forwarding_endpoint",
            module="pem",
            object_types=("forwarding-endpoint",),
        ),
        header_types=(("pem", "forwarding-endpoint"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="persistence",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="fallback",
                        value_type="enum",
                        in_sections=("persistence",),
                        enum_values=("destination-ip", "disabled", "source-ip"),
                    ),
                    BigipPropertySpec(
                        name="hash-settings",
                        value_type="list",
                        in_sections=("persistence",),
                    ),
                    BigipPropertySpec(
                        name="type",
                        value_type="enum",
                        in_sections=("persistence",),
                        enum_values=("destination-ip", "disabled", "hash", "source-ip"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="fallback",
                value_type="enum",
                in_sections=("persistence",),
                enum_values=("destination-ip", "disabled", "source-ip"),
            ),
            BigipPropertySpec(
                name="hash-settings",
                value_type="list",
                in_sections=("persistence",),
                block=(
                    BigipPropertySpec(
                        name="algorithm",
                        value_type="unknown",
                        in_sections=("persistence", "hash-settings"),
                    ),
                    BigipPropertySpec(
                        name="length",
                        value_type="integer",
                        in_sections=("persistence", "hash-settings"),
                    ),
                    BigipPropertySpec(
                        name="offset",
                        value_type="integer",
                        in_sections=("persistence", "hash-settings"),
                    ),
                    BigipPropertySpec(
                        name="source",
                        value_type="enum",
                        in_sections=("persistence", "hash-settings"),
                        enum_values=("tcl-snippet", "uri"),
                    ),
                    BigipPropertySpec(
                        name="tcl-value",
                        value_type="string",
                        in_sections=("persistence", "hash-settings"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="algorithm",
                value_type="unknown",
                in_sections=("persistence", "hash-settings"),
            ),
            BigipPropertySpec(
                name="length",
                value_type="integer",
                in_sections=("persistence", "hash-settings"),
            ),
            BigipPropertySpec(
                name="offset",
                value_type="integer",
                in_sections=("persistence", "hash-settings"),
            ),
            BigipPropertySpec(
                name="source",
                value_type="enum",
                in_sections=("persistence", "hash-settings"),
                enum_values=("tcl-snippet", "uri"),
            ),
            BigipPropertySpec(
                name="tcl-value",
                value_type="string",
                in_sections=("persistence", "hash-settings"),
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                in_sections=("persistence",),
                enum_values=("destination-ip", "disabled", "hash", "source-ip"),
            ),
            BigipPropertySpec(
                name="pool",
                value_type="reference",
                references=(
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ),
            ),
            BigipPropertySpec(name="snat-pool", value_type="reference", default="none"),
            BigipPropertySpec(
                name="source-port",
                value_type="enum",
                enum_values=("change", "preserve", "preserve-strict"),
                default="preserve",
            ),
            BigipPropertySpec(
                name="translate-address",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="translate-service",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
        ),
    )
