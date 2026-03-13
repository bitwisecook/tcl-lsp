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
            "security_dos_bot_signature",
            module="security",
            object_types=("dos bot-signature",),
        ),
        header_types=(("security", "dos bot-signature"),),
        properties=(
            BigipPropertySpec(name="category", value_type="string"),
            BigipPropertySpec(
                name="domains",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="risk", value_type="enum", enum_values=("high", "low", "medium")
            ),
            BigipPropertySpec(name="rule", value_type="string"),
            BigipPropertySpec(name="signature-references", value_type="string"),
            BigipPropertySpec(name="url", value_type="string"),
            BigipPropertySpec(
                name="match-type",
                value_type="enum",
                in_sections=("url",),
                enum_values=("contains", "regexp"),
            ),
            BigipPropertySpec(name="search-string", value_type="string", in_sections=("url",)),
            BigipPropertySpec(name="user-agent", value_type="string"),
            BigipPropertySpec(
                name="match-type",
                value_type="enum",
                in_sections=("user-agent",),
                enum_values=("contains", "regexp"),
            ),
            BigipPropertySpec(
                name="search-string", value_type="string", in_sections=("user-agent",)
            ),
        ),
    )
