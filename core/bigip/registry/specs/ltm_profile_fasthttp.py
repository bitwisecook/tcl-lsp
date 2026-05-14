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
            "ltm_profile_fasthttp",
            module="ltm",
            object_types=("profile fasthttp",),
        ),
        header_types=(("ltm", "profile fasthttp"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="client-close-timeout", value_type="integer", default="5"),
            BigipPropertySpec(
                name="connpool-idle-timeout-override",
                value_type="integer",
                default="0 (zero) seconds, which disables the override setting",
            ),
            BigipPropertySpec(name="connpool-max-reuse", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="connpool-max-size", value_type="integer", default="2048"),
            BigipPropertySpec(name="connpool-min-size", value_type="integer"),
            BigipPropertySpec(
                name="connpool-replenish",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="connpool-step", value_type="integer", default="4"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_fasthttp",),
                default="fasthttp",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="force-http-10-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="hardware-syn-cookie",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(name="header-insert", value_type="string", default="none"),
            BigipPropertySpec(
                name="http-11-close-workarounds",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="idle-timeout", value_type="integer", default="300 seconds"),
            BigipPropertySpec(
                name="insert-xforwarded-for",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="layer-7",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="max-header-size", value_type="integer", default="32768"),
            BigipPropertySpec(name="max-requests", value_type="integer"),
            BigipPropertySpec(
                name="mss-override",
                value_type="integer",
                default="0 (zero), which corresponds to an MSS of 1460",
            ),
            BigipPropertySpec(name="receive-window-size", value_type="unknown"),
            BigipPropertySpec(
                name="reset-on-timeout",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="server-close-timeout", value_type="integer", default="5"),
            BigipPropertySpec(
                name="server-sack",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="server-timestamp",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="unclean-shutdown",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
        ),
    )
