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
            BigipPropertySpec(name="cur-pending-req-sweeper-interval", value_type="integer"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_message_routing_generic_protocol",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="disable-parser", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="max-egress-buffer", value_type="integer"),
            BigipPropertySpec(name="max-message-size", value_type="integer"),
            BigipPropertySpec(name="message-terminator", value_type="string"),
            BigipPropertySpec(name="no-response", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="transaction-timeout", value_type="integer"),
        ),
    )
