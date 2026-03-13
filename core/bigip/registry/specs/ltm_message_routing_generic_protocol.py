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
            "ltm_message_routing_generic_protocol",
            module="ltm",
            object_types=("message-routing generic protocol",),
        ),
        header_types=(("ltm", "message-routing generic protocol"),),
        properties=(
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="disable-parser", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="max-egress-buffer", value_type="integer"),
            BigipPropertySpec(name="max-message-size", value_type="integer"),
            BigipPropertySpec(name="message-terminator", value_type="string"),
            BigipPropertySpec(name="no-response", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="transaction-timeout", value_type="integer"),
            BigipPropertySpec(name="cur-pending-req-sweeper-interval", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
