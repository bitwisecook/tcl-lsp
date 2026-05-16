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
            "security_protocol_inspection_signature",
            module="security",
            object_types=("protocol-inspection signature",),
        ),
        header_types=(("security", "protocol-inspection signature"),),
        properties=(
            BigipPropertySpec(
                name="accuracy",
                value_type="enum",
                enum_values=("high", "low", "medium"),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                enum_values=("accept", "drop", "reject"),
            ),
            BigipPropertySpec(name="app-service", value_type="string"),
            BigipPropertySpec(name="attack-type", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="direction",
                value_type="enum",
                enum_values=("any", "to-client", "to-server"),
            ),
            BigipPropertySpec(name="documentation", value_type="string"),
            BigipPropertySpec(name="id", value_type="integer"),
            BigipPropertySpec(name="last-updated", value_type="unknown"),
            BigipPropertySpec(
                name="log",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="performance-impact",
                value_type="enum",
                enum_values=("high", "low", "medium"),
            ),
            BigipPropertySpec(
                name="protocol",
                value_type="enum",
                enum_values=("any", "tcp", "udp"),
            ),
            BigipPropertySpec(name="reference-links", value_type="string"),
            BigipPropertySpec(name="references", value_type="string"),
            BigipPropertySpec(name="revision", value_type="integer"),
            BigipPropertySpec(
                name="risk",
                value_type="enum",
                enum_values=("critical", "high", "low", "medium"),
            ),
            BigipPropertySpec(name="service", value_type="string"),
            BigipPropertySpec(name="sig", value_type="unknown"),
            BigipPropertySpec(name="systems", value_type="string"),
            BigipPropertySpec(
                name="user-defined",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
        ),
    )
