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
                name="allow",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                default="all",
            ),
            BigipPropertySpec(
                name="banner",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="banner-text", value_type="string"),
            BigipPropertySpec(
                name="inactivity-timeout",
                value_type="integer",
                default="0 (zero) seconds, which indicates that inactivity timeout is disabled",
            ),
            BigipPropertySpec(name="include", value_type="string"),
            BigipPropertySpec(
                name="log-level",
                value_type="enum",
                enum_values=(
                    "debug",
                    "debug1",
                    "debug2",
                    "debug3",
                    "error",
                    "fatal",
                    "info",
                    "quiet",
                    "verbose",
                ),
            ),
            BigipPropertySpec(
                name="login",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="port", value_type="integer", default="22"),
            BigipPropertySpec(
                name="LoginGraceTime",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="MACs",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="MaxAuthTries",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="MaxStartups",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="Protocol",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(name='"', value_type="string", usage_flags=frozenset(("read_only",))),
        ),
    )
