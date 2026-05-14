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
            "apm_policy_agent_logon_page",
            module="apm",
            object_types=("policy agent logon-page",),
        ),
        header_types=(("apm", "policy agent logon-page"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="basic-auth-realm", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="clean-sess-var1",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="clean-sess-var2",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="clean-sess-var3",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="clean-sess-var4",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="clean-sess-var5",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="customization-group", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="field-modifiable1",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="field-modifiable2",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="field-modifiable3",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="field-modifiable4",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="field-modifiable5",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="field-type1",
                value_type="enum",
                allow_none=True,
                enum_values=("checkbox", "none", "password", "text"),
            ),
            BigipPropertySpec(
                name="field-type2",
                value_type="enum",
                allow_none=True,
                enum_values=("checkbox", "none", "password", "text"),
            ),
            BigipPropertySpec(
                name="field-type3",
                value_type="enum",
                allow_none=True,
                enum_values=("checkbox", "none", "password", "text"),
            ),
            BigipPropertySpec(
                name="field-type4",
                value_type="enum",
                allow_none=True,
                enum_values=("checkbox", "none", "password", "text"),
            ),
            BigipPropertySpec(
                name="field-type5",
                value_type="enum",
                allow_none=True,
                enum_values=("checkbox", "none", "password", "text"),
            ),
            BigipPropertySpec(
                name="http-401-auth-level",
                value_type="enum",
                allow_none=True,
                enum_values=("basic", "basic-negotiate", "negotiate", "none"),
            ),
            BigipPropertySpec(name="post-var-name1", value_type="integer", allow_none=True),
            BigipPropertySpec(name="post-var-name2", value_type="integer", allow_none=True),
            BigipPropertySpec(name="post-var-name3", value_type="integer", allow_none=True),
            BigipPropertySpec(name="post-var-name4", value_type="integer", allow_none=True),
            BigipPropertySpec(name="post-var-name5", value_type="integer", allow_none=True),
            BigipPropertySpec(name="session-var-name1", value_type="integer", allow_none=True),
            BigipPropertySpec(name="session-var-name2", value_type="integer", allow_none=True),
            BigipPropertySpec(name="session-var-name3", value_type="integer", allow_none=True),
            BigipPropertySpec(name="session-var-name4", value_type="integer", allow_none=True),
            BigipPropertySpec(name="session-var-name5", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="split-username",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="type", value_type="enum", enum_values=("401", "form-based")),
        ),
    )
