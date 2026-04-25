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
            "ltm_profile_quic",
            module="ltm",
            object_types=("profile quic",),
        ),
        header_types=(("ltm", "profile quic"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_quic",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="bidi-concurrent-streams-per-connection", value_type="integer"),
            BigipPropertySpec(
                name="spin-bit", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="uni-concurrent-streams-per-connection", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
