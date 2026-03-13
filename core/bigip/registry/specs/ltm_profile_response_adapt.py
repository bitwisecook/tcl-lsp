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
            "ltm_profile_response_adapt",
            module="ltm",
            object_types=("profile response-adapt",),
        ),
        header_types=(("ltm", "profile response-adapt"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_response_adapt",),
            ),
            BigipPropertySpec(name="enabled", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="internal-virtual", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="preview-size", value_type="integer"),
            BigipPropertySpec(
                name="service-down-action",
                value_type="enum",
                enum_values=("ignore", "reset", "drop"),
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="allow-http-10", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
