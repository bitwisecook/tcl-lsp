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
            "sys_management_proxy_config",
            module="sys",
            object_types=("management-proxy-config",),
        ),
        header_types=(("sys", "management-proxy-config"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="proxy-ip-addr", value_type="string"),
            BigipPropertySpec(
                name="proxy-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="username", value_type="reference", references=("auth_user",)),
            BigipPropertySpec(name="password", value_type="string"),
        ),
    )
