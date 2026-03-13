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
            "ltm_auth_tacacs",
            module="ltm",
            object_types=("auth tacacs",),
        ),
        header_types=(("ltm", "auth tacacs"),),
        properties=(
            BigipPropertySpec(
                name="accounting",
                value_type="enum",
                enum_values=("send-to-all-servers", "send-to-first-server"),
            ),
            BigipPropertySpec(
                name="authentication",
                value_type="enum",
                enum_values=("use-all-servers", "use-first-server"),
            ),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="encryption", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="protocol",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "protocol"),
            ),
            BigipPropertySpec(name="secret", value_type="string"),
            BigipPropertySpec(name="service", value_type="boolean", allow_none=True),
        ),
    )
