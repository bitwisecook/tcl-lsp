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
            "cli_preference",
            module="cli",
            object_types=("preference",),
        ),
        header_types=(("cli", "preference"),),
        properties=(
            BigipPropertySpec(name="alias-path", value_type="unknown"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="confirm-edit",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="display-threshold", value_type="integer"),
            BigipPropertySpec(name="editor", value_type="enum", enum_values=("nano", "vi")),
            BigipPropertySpec(
                name="fully-qualified-host",
                value_type="unknown",
                default="to not display this information",
            ),
            BigipPropertySpec(
                name="history-date-time",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="history-file-size", value_type="integer"),
            BigipPropertySpec(name="history-size", value_type="integer", default="500"),
            BigipPropertySpec(
                name="keymap",
                value_type="enum",
                enum_values=("default", "emacs", "vi"),
                default="default",
            ),
            BigipPropertySpec(
                name="list-all-properties",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="mcp-state",
                value_type="unknown",
                allow_none=True,
                default="to not display this information",
            ),
            BigipPropertySpec(
                name="pager",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="prompt", value_type="unknown", shape_kind="object"),
            BigipPropertySpec(
                name="show-aliases",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="stat-units",
                value_type="enum",
                enum_values=(
                    "default",
                    "exa",
                    "gig",
                    "kil",
                    "meg",
                    "peta",
                    "raw",
                    "tera",
                    "yotta",
                    "zetta",
                ),
            ),
            BigipPropertySpec(
                name="suppress-warnings",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "config-version", "none"),
                default="none",
            ),
            BigipPropertySpec(name="table-indent-width", value_type="integer"),
            BigipPropertySpec(
                name="tcl-syntax-highlighting",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="video",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="warn",
                value_type="enum",
                enum_values=("bell", "disabled", "visual-bell"),
                default="bell",
            ),
        ),
    )
