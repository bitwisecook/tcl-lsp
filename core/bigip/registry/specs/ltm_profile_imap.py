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
            "ltm_profile_imap",
            module="ltm",
            object_types=("profile imap",),
        ),
        header_types=(("ltm", "profile imap"),),
        properties=(
            BigipPropertySpec(
                name="activation-mode",
                value_type="enum",
                allow_none=True,
                enum_values=("allow", "none", "require"),
                default="REQUIRE",
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
                allow_none=True,
                references=("ltm_profile_imap",),
                default="imap",
            ),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
