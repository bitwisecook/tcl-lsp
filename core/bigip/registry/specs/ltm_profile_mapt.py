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
            "ltm_profile_mapt",
            module="ltm",
            object_types=("profile mapt",),
        ),
        header_types=(("ltm", "profile mapt"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_mapt",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ip6-prefix", value_type="string"),
            BigipPropertySpec(name="ip4-prefix", value_type="string"),
            BigipPropertySpec(name="ea-bits-length", value_type="integer"),
            BigipPropertySpec(name="port-offset", value_type="integer"),
            BigipPropertySpec(name="br-prefix", value_type="string"),
        ),
    )
