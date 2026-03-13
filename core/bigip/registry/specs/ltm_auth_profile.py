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
            "ltm_auth_profile",
            module="ltm",
            object_types=("auth profile",),
        ),
        header_types=(("ltm", "auth profile"),),
        properties=(
            BigipPropertySpec(name="configuration", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="cookie-key", value_type="string"),
            BigipPropertySpec(name="cookie-name", value_type="string"),
            BigipPropertySpec(name="credential-source", value_type="string"),
            BigipPropertySpec(name="defaults-from", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="enabled", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(name="rule", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
