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
            "ltm_profile_stream",
            module="ltm",
            object_types=("profile stream",),
        ),
        header_types=(("ltm", "profile stream"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_stream",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="source", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="target", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="chunking-enabled", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="chunk-size", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
