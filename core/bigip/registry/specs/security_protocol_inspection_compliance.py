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
            "security_protocol_inspection_compliance",
            module="security",
            object_types=("protocol-inspection compliance",),
        ),
        header_types=(("security", "protocol-inspection compliance"),),
        properties=(
            BigipPropertySpec(
                name="accuracy", value_type="enum", enum_values=("high", "low", "medium")
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="service", value_type="string"),
            BigipPropertySpec(
                name="action", value_type="enum", enum_values=("accept", "drop", "reject")
            ),
            BigipPropertySpec(
                name="direction", value_type="enum", enum_values=("any", "to-client", "to-server")
            ),
            BigipPropertySpec(name="log", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="documentation", value_type="string"),
            BigipPropertySpec(
                name="performance-impact", value_type="enum", enum_values=("high", "low", "medium")
            ),
            BigipPropertySpec(name="systems", value_type="string"),
            BigipPropertySpec(name="attack-type", value_type="string"),
            BigipPropertySpec(name="id", value_type="integer"),
            BigipPropertySpec(
                name="protocol", value_type="enum", enum_values=("any", "tcp", "udp")
            ),
            BigipPropertySpec(
                name="risk", value_type="enum", enum_values=("critical", "high", "low", "medium")
            ),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(name="value-type", value_type="string"),
        ),
    )
