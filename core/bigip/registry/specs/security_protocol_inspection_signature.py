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
                name="accuracy", value_type="enum", enum_values=("high", "low", "medium")
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="last-updated", value_type="string"),
            BigipPropertySpec(name="reference-links", value_type="string"),
            BigipPropertySpec(name="service", value_type="string"),
            BigipPropertySpec(
                name="action", value_type="enum", enum_values=("accept", "drop", "reject")
            ),
            BigipPropertySpec(
                name="direction", value_type="enum", enum_values=("any", "to-client", "to-server")
            ),
            BigipPropertySpec(name="log", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="references", value_type="string"),
            BigipPropertySpec(name="sig", value_type="boolean"),
            BigipPropertySpec(name="documentation", value_type="string"),
            BigipPropertySpec(
                name="performance-impact", value_type="enum", enum_values=("high", "low", "medium")
            ),
            BigipPropertySpec(name="revision", value_type="integer"),
            BigipPropertySpec(name="systems", value_type="string"),
            BigipPropertySpec(name="attack-type", value_type="string"),
            BigipPropertySpec(name="id", value_type="integer"),
            BigipPropertySpec(
                name="protocol", value_type="enum", enum_values=("any", "tcp", "udp")
            ),
            BigipPropertySpec(
                name="risk", value_type="enum", enum_values=("critical", "high", "low", "medium")
            ),
            BigipPropertySpec(name="user-defined", value_type="enum", enum_values=("yes", "no")),
        ),
    )
