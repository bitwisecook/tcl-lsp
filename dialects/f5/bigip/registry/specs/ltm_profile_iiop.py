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
                name="abort-on-timeout",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_profile_iiop",),
                default="iiop",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="persist-object-key",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="persist-request-id",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="timeout", value_type="integer", default="30 seconds"),
        ),
    )
