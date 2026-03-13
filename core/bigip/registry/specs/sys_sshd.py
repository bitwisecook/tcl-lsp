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
            "sys_sshd",
            module="sys",
            object_types=("sshd",),
        ),
        header_types=(("sys", "sshd"),),
        properties=(
            BigipPropertySpec(
                name="allow", value_type="enum", enum_values=("add", "delete", "replace-all-with")
            ),
            BigipPropertySpec(
                name="banner", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="banner-text", value_type="string"),
            BigipPropertySpec(name="inactivity-timeout", value_type="integer"),
            BigipPropertySpec(name="include", value_type="string"),
            BigipPropertySpec(name="login", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="log-level", value_type="string"),
            BigipPropertySpec(name="info", value_type="string"),
            BigipPropertySpec(name="port", value_type="integer", min_value=0, max_value=65535),
        ),
    )
