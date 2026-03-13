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
                name="defaults-from", value_type="reference", references=("ltm_profile_mssql",)
            ),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="read-pool", value_type="reference", references=("ltm_pool",)),
            BigipPropertySpec(
                name="read-write-split-by-user",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="read-write-split-by-command",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="user-can-write-by-default", value_type="enum", enum_values=("true", "false")
            ),
            BigipPropertySpec(
                name="user-list",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="write-persist-timer", value_type="string"),
            BigipPropertySpec(name="write-pool", value_type="reference", references=("ltm_pool",)),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
