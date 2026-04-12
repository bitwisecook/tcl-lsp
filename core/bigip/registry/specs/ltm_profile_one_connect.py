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
            "ltm_profile_one_connect",
            module="ltm",
            object_types=("profile one-connect",),
        ),
        header_types=(("ltm", "profile one-connect"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_profile_one_connect",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="idle-timeout-override", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="share-pools", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="max-age", value_type="integer"),
            BigipPropertySpec(name="max-reuse", value_type="integer"),
            BigipPropertySpec(name="max-size", value_type="integer"),
            BigipPropertySpec(name="source-mask", value_type="string"),
            BigipPropertySpec(
                name="limit-type",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "idle", "strict"),
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
