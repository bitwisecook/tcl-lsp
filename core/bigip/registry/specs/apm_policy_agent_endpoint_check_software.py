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
            "apm_policy_agent_endpoint_check_software",
            module="apm",
            object_types=("policy agent endpoint-check-software",),
        ),
        header_types=(("apm", "policy agent endpoint-check-software"),),
        properties=(
            BigipPropertySpec(
                name="check-list-type",
                value_type="enum",
                enum_values=("allow", "deny", "required"),
            ),
            BigipPropertySpec(
                name="collect",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="continuous-check",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="items",
                value_type="enum",
                enum_values=(
                    "db-age",
                    "db-version",
                    "last-scan",
                    "missing-updates",
                    "platform",
                    "product_id",
                    "state",
                    "vendor_id",
                    "version",
                ),
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=(
                    "antispyware",
                    "antivirus",
                    "firewall",
                    "hard-disk-encryption",
                    "health-agent",
                    "patch-management",
                    "peer-to-peer",
                ),
            ),
        ),
    )
