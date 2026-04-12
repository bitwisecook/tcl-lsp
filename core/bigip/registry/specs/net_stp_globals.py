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
            BigipPropertySpec(name="config-name", value_type="string"),
            BigipPropertySpec(name="config-revision", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="fwd-delay", value_type="integer"),
            BigipPropertySpec(name="hello-time", value_type="integer"),
            BigipPropertySpec(name="max-age", value_type="integer"),
            BigipPropertySpec(name="max-hops", value_type="integer"),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=("disabled", "mstp", "passthru", "rstp", "stp"),
            ),
            BigipPropertySpec(name="transmit-hold", value_type="integer"),
        ),
    )
