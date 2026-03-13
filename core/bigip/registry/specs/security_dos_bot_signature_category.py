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
            "security_dos_bot_signature_category",
            module="security",
            object_types=("dos bot-signature-category",),
        ),
        header_types=(("security", "dos bot-signature-category"),),
        properties=(
            BigipPropertySpec(name="type", value_type="enum", enum_values=("benign", "malicious")),
        ),
    )
