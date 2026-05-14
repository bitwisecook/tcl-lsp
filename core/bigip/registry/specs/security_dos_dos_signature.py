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
            "security_dos_dos_signature",
            module="security",
            object_types=("dos dos-signature",),
        ),
        header_types=(("security", "dos dos-signature"),),
        properties=(
            BigipPropertySpec(name="alias", value_type="string"),
            BigipPropertySpec(
                name="app-service",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="approval-state",
                value_type="enum",
                enum_values=("manually-approved", "unapproved"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="family",
                value_type="enum",
                enum_values=("dns", "http", "network", "tls"),
            ),
            BigipPropertySpec(
                name="hardware-offload",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="manual-detection-threshold", value_type="integer"),
            BigipPropertySpec(name="manual-mitigation-threshold", value_type="integer"),
            BigipPropertySpec(name="multiplier-mitigation-percentage", value_type="integer"),
            BigipPropertySpec(
                name="origin",
                value_type="enum",
                enum_values=("dynamic-bdos", "user-defined"),
            ),
            BigipPropertySpec(name="parent-context", value_type="string"),
            BigipPropertySpec(
                name="parent-context-type",
                value_type="enum",
                enum_values=("device", "device-netflow", "virtual-server"),
            ),
            BigipPropertySpec(name="parent-profile", value_type="string"),
            BigipPropertySpec(name="predicates", value_type="unknown"),
            BigipPropertySpec(
                name="shareability-state",
                value_type="enum",
                enum_values=("fully-shareable", "not-shareable"),
            ),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                enum_values=("detect-only", "disabled", "learn-only", "mitigate"),
            ),
            BigipPropertySpec(name="tags", value_type="unknown"),
            BigipPropertySpec(
                name="threshold-mode",
                value_type="enum",
                enum_values=(
                    "fully-automatic",
                    "manual",
                    "manual-multiplier-mitigation",
                    "stress-based-mitigation",
                ),
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=("dynamic", "persistent"),
            ),
        ),
    )
