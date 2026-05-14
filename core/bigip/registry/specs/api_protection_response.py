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
            "api_protection_response",
            module="api-protection",
            object_types=("response",),
        ),
        header_types=(("api-protection", "response"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="body", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="headers",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("headers",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="header-name",
                value_type="string",
                in_sections=("headers",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="header-value",
                value_type="string",
                in_sections=("headers",),
                allow_none=True,
            ),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(name="status-code", value_type="string"),
            BigipPropertySpec(name="status-string", value_type="string"),
        ),
    )
