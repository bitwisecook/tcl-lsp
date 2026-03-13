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
            "sys_application_apl_script",
            module="sys",
            object_types=("application apl-script",),
        ),
        header_types=(("sys", "application apl-script"),),
        properties=(
            BigipPropertySpec(name="apl-checksum", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="apl-signature", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="ignore-verification", value_type="enum", enum_values=("true", "false")
            ),
            BigipPropertySpec(name="generate", value_type="string"),
            BigipPropertySpec(name="sys", value_type="string"),
            BigipPropertySpec(name="define", value_type="string", in_sections=("sys",)),
            BigipPropertySpec(name="actions", value_type="string", in_sections=("sys",)),
            BigipPropertySpec(name="definition", value_type="string", in_sections=("actions",)),
            BigipPropertySpec(
                name="presentation", value_type="string", in_sections=("definition",)
            ),
            BigipPropertySpec(name="include", value_type="string", in_sections=("presentation",)),
            BigipPropertySpec(name="section", value_type="string", in_sections=("presentation",)),
            BigipPropertySpec(name="string", value_type="string", in_sections=("section",)),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("section",),
                min_value=0,
                max_value=65535,
            ),
        ),
    )
