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
            "apm_aaa_active_directory_trusted_domains",
            module="apm",
            object_types=("aaa active-directory-trusted-domains",),
        ),
        header_types=(("apm", "aaa active-directory-trusted-domains"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="root-domain", value_type="string"),
            BigipPropertySpec(
                name="trusted-domains",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="active-directory",
                value_type="reference",
                in_sections=("trusted-domains",),
                references=("apm_aaa_active_directory",),
            ),
        ),
    )
