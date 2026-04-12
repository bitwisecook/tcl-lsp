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
            "ltm_profile_iiop",
            module="ltm",
            object_types=("profile iiop",),
        ),
        header_types=(("ltm", "profile iiop"),),
        properties=(
            BigipPropertySpec(
                name="abort-on-timeout", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_profile_iiop",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="persist-object-key", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="persist-request-id", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
