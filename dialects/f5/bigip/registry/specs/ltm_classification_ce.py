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
            "ltm_classification_ce",
            module="ltm",
            object_types=("classification ce",),
        ),
        header_types=(("ltm", "classification ce"),),
        properties=(
            BigipPropertySpec(
                name="allow-reclassification",
                value_type="enum",
                enum_values=("off", "on"),
            ),
            BigipPropertySpec(name="analyze-dns", value_type="enum", enum_values=("off", "on")),
            BigipPropertySpec(
                name="analyze-ssl-serverside",
                value_type="enum",
                enum_values=("off", "on"),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="cache-results", value_type="enum", enum_values=("off", "on")),
            BigipPropertySpec(name="flow-bundling", value_type="enum", enum_values=("off", "on")),
            BigipPropertySpec(
                name="policies",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
        ),
    )
