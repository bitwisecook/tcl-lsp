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
            "auth_tacacs",
            module="auth",
            object_types=("tacacs",),
        ),
        header_types=(("auth", "tacacs"),),
        properties=(
            BigipPropertySpec(
                name="accounting",
                value_type="enum",
                enum_values=("send-to-first-server", "send-to-all-servers"),
            ),
            BigipPropertySpec(
                name="authentication",
                value_type="enum",
                enum_values=("use-first-server", "use-all-servers"),
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
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
