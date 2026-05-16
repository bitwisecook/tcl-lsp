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
            "sys_smtp_server",
            module="sys",
            object_types=("smtp-server",),
        ),
        header_types=(("sys", "smtp-server"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="encrypted-connection",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "ssl", "tls"),
                default="none",
            ),
            BigipPropertySpec(name="from-address", value_type="string"),
            BigipPropertySpec(name="local-host-name", value_type="string"),
            BigipPropertySpec(name="password", value_type="string"),
            BigipPropertySpec(name="smtp-server-host-name", value_type="string"),
            BigipPropertySpec(name="smtp-server-port", value_type="integer", default="25"),
            BigipPropertySpec(name="username", value_type="string"),
        ),
    )
