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
            "ltm_profile_one_connect",
            module="ltm",
            object_types=("profile one-connect",),
        ),
        header_types=(("ltm", "profile one-connect"),),
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
                references=("ltm_profile_one_connect",),
                default="oneconnect",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="idle-timeout-override",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="limit-type",
                value_type="enum",
                allow_none=True,
                enum_values=("idle", "none", "strict"),
            ),
            BigipPropertySpec(name="max-age", value_type="integer", default="86400"),
            BigipPropertySpec(name="max-reuse", value_type="integer", default="1000"),
            BigipPropertySpec(name="max-size", value_type="integer", default="10000"),
            BigipPropertySpec(
                name="share-pools",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="source-mask",
                value_type="string",
                shape_kind="ip-address",
                default="0",
            ),
        ),
    )
