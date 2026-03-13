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
                name="allow-reclassification", value_type="enum", enum_values=("on", "off")
            ),
            BigipPropertySpec(name="analyze-dns", value_type="enum", enum_values=("on", "off")),
            BigipPropertySpec(
                name="analyze-ssl-serverside", value_type="enum", enum_values=("on", "off")
            ),
            BigipPropertySpec(name="flow-bundling", value_type="enum", enum_values=("on", "off")),
            BigipPropertySpec(name="cache-results", value_type="enum", enum_values=("on", "off")),
            BigipPropertySpec(
                name="policies",
                value_type="reference",
                allow_none=True,
                enum_values=("add", "delete", "default", "replace-all-with", "none"),
                references=("ltm_policy",),
            ),
        ),
    )
