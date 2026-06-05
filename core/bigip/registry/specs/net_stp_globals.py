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
            "net_stp_globals",
            module="net",
            object_types=("stp-globals",),
        ),
        header_types=(("net", "stp-globals"),),
        properties=(
            BigipPropertySpec(name="config-name", value_type="reference"),
            BigipPropertySpec(name="config-revision", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="fwd-delay",
                value_type="integer",
                default="15 seconds, and the valid range is 4 to 30",
            ),
            BigipPropertySpec(
                name="hello-time",
                value_type="integer",
                default="2 seconds, and the valid range is 1 - 10",
            ),
            BigipPropertySpec(
                name="max-age",
                value_type="integer",
                default="20 seconds, and the valid range is 6-40 seconds",
            ),
            BigipPropertySpec(name="max-hops", value_type="integer"),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=("disabled", "mstp", "passthru", "rstp", "stp"),
            ),
            BigipPropertySpec(name="transmit-hold", value_type="integer"),
        ),
    )
