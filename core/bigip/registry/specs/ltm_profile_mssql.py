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
            "ltm_profile_mssql",
            module="ltm",
            object_types=("profile mssql",),
        ),
        header_types=(("ltm", "profile mssql"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_profile_mssql",),
                default="mssql",
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="read-pool", value_type="string"),
            BigipPropertySpec(
                name="read-write-split-by-command",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="read-write-split-by-user",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="user-can-write-by-default",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="user-list",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                default="true, or the users who have write access to the MS SQL database if user-can-write-by-default is false",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(name="write-persist-timer", value_type="unknown"),
            BigipPropertySpec(
                name="write-pool",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
