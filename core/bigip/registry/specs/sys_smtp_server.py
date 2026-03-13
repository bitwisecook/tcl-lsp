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
                name="encrypted-connection",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "tls", "ssl"),
            ),
            BigipPropertySpec(name="local-host-name", value_type="string"),
            BigipPropertySpec(name="smtp-server-host-name", value_type="string"),
            BigipPropertySpec(
                name="smtp-server-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(
                name="from-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="username", value_type="reference", references=("auth_user",)),
            BigipPropertySpec(name="password", value_type="string"),
        ),
    )
