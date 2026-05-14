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
            "apm_aaa_oauth_request",
            module="apm",
            object_types=("aaa oauth-request",),
        ),
        header_types=(("apm", "aaa oauth-request"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="headers",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="value", value_type="unknown", in_sections=("headers",)),
            BigipPropertySpec(name="method", value_type="enum", enum_values=("get", "post")),
            BigipPropertySpec(
                name="parameters",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="type", value_type="unknown", in_sections=("parameters",)),
            BigipPropertySpec(
                name="value",
                value_type="string",
                in_sections=("parameters",),
                allow_none=True,
            ),
            BigipPropertySpec(name="type", value_type="unknown"),
            BigipPropertySpec(name="uri", value_type="string", allow_none=True),
        ),
    )
