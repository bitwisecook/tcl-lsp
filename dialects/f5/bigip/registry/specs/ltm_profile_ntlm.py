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
            "ltm_profile_ntlm",
            module="ltm",
            object_types=("profile ntlm",),
        ),
        header_types=(("ltm", "profile ntlm"),),
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
                references=("ltm_profile_ntlm",),
                default="ntlm",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="insert-cookie-domain",
                value_type="unknown",
                default="none, which causes no domain to be configured for the inserted cookie",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="insert-cookie-name",
                value_type="reference",
                default="NTLMconnpool",
            ),
            BigipPropertySpec(
                name="insert-cookie-passphrase",
                value_type="unknown",
                default="mypassphrase",
            ),
            BigipPropertySpec(
                name="key-by-cookie",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="key-by-cookie-name",
                value_type="reference",
                default="mycookie",
            ),
            BigipPropertySpec(
                name="key-by-domain",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="key-by-ip-address",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="key-by-target",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="key-by-user",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="key-by-workstation",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
        ),
    )
