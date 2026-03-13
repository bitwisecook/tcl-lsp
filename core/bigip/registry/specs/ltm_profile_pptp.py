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
            "ltm_profile_pptp",
            module="ltm",
            object_types=("profile pptp",),
        ),
        header_types=(("ltm", "profile pptp"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_pptp",),
            ),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="publisher-name", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="include-destination-ip",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="csv-format", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
