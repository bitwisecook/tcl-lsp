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
            "ltm_profile_fix",
            module="ltm",
            object_types=("profile fix",),
        ),
        header_types=(("ltm", "profile fix"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_fix",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="error-action",
                value_type="enum",
                enum_values=("drop_connection", "dont_forward"),
            ),
            BigipPropertySpec(
                name="full-logon-parsing", value_type="enum", enum_values=("true", "false")
            ),
            BigipPropertySpec(name="message-log-publisher", value_type="string"),
            BigipPropertySpec(
                name="quick-parsing", value_type="enum", enum_values=("true", "false")
            ),
            BigipPropertySpec(name="statistics-sample-interval", value_type="integer"),
            BigipPropertySpec(name="report-log-publisher", value_type="string"),
            BigipPropertySpec(
                name="response-parsing", value_type="enum", enum_values=("true", "false")
            ),
            BigipPropertySpec(name="sender-tag-class", value_type="list", repeated=True),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
