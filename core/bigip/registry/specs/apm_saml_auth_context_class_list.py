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
            "apm_saml_auth_context_class_list",
            module="apm",
            object_types=("saml auth-context-class-list",),
        ),
        header_types=(("apm", "saml auth-context-class-list"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="classes",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(name="order", value_type="integer", in_sections=("classes",)),
                    BigipPropertySpec(name="value", value_type="string", in_sections=("classes",)),
                ),
            ),
            BigipPropertySpec(name="order", value_type="integer", in_sections=("classes",)),
            BigipPropertySpec(name="value", value_type="string", in_sections=("classes",)),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
        ),
    )
