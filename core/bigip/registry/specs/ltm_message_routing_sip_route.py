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
            "ltm_message_routing_sip_route",
            module="ltm",
            object_types=("message-routing sip route",),
        ),
        header_types=(("ltm", "message-routing sip route"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="from-uri", value_type="string"),
            BigipPropertySpec(
                name="peer-selection-mode", value_type="enum", enum_values=("ratio", "sequential")
            ),
            BigipPropertySpec(name="peers", value_type="boolean", repeated=True, allow_none=True),
            BigipPropertySpec(name="request-uri", value_type="string"),
            BigipPropertySpec(name="to-uri", value_type="string"),
            BigipPropertySpec(name="virtual-server", value_type="string"),
        ),
    )
