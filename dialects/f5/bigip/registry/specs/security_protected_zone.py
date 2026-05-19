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
            "security_protected_zone",
            module="security",
            object_types=("protected zone",),
        ),
        header_types=(("security", "protected zone"),),
        properties=(
            BigipPropertySpec(name="copy-from", value_type="string"),
            BigipPropertySpec(
                name="dos-profile",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
                references=("security_dos_profile",),
            ),
            BigipPropertySpec(name="fw-zone-name", value_type="unknown"),
            BigipPropertySpec(
                name="log-profile",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
        ),
    )
