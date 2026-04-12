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
            "ltm_profile_web_acceleration",
            module="ltm",
            object_types=("profile web-acceleration",),
        ),
        header_types=(("ltm", "profile web-acceleration"),),
        properties=(
            BigipPropertySpec(name="cache-aging-rate", value_type="integer"),
            BigipPropertySpec(
                name="cache-client-cache-control-mode",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "max-age", "none"),
            ),
            BigipPropertySpec(
                name="cache-insert-age-header",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="cache-max-age", value_type="integer"),
            BigipPropertySpec(name="cache-max-entries", value_type="integer"),
            BigipPropertySpec(name="cache-object-max-size", value_type="integer"),
            BigipPropertySpec(name="cache-object-min-size", value_type="integer"),
            BigipPropertySpec(name="cache-size", value_type="integer"),
            BigipPropertySpec(name="cache-uri-exclude", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="cache-uri-include", value_type="string"),
            BigipPropertySpec(
                name="cache-uri-include-override", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(name="cache-uri-pinned", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_web_acceleration",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
