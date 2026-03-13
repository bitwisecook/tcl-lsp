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
            "ltm_profile_ocsp",
            module="ltm",
            object_types=("profile ocsp",),
        ),
        header_types=(("ltm", "profile ocsp"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_ocsp",),
            ),
            BigipPropertySpec(name="max-age", value_type="integer"),
            BigipPropertySpec(name="nonce", value_type="enum", enum_values=("enabled", "disabled")),
        ),
    )
