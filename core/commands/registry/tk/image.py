"""image -- Create and manipulate images."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page image.n"
_av = make_av(_SOURCE)


@register
class ImageCommand(CommandDef):
    name = "image"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="image",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate images.",
                synopsis=(
                    "image create type ?name? ?option value ...?",
                    "image delete ?name name ...?",
                    "image height name",
                    "image inuse name",
                    "image names",
                    "image type name",
                    "image types",
                    "image width name",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="image option ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "create",
                                "Create a new image of the given type (photo or bitmap).",
                                "image create type ?name? ?option value ...?",
                            ),
                            _av(
                                "delete",
                                "Delete one or more images by name.",
                                "image delete ?name name ...?",
                            ),
                            _av(
                                "height",
                                "Return the height of the image in pixels.",
                                "image height name",
                            ),
                            _av(
                                "inuse",
                                "Return whether the image is in use by any widgets.",
                                "image inuse name",
                            ),
                            _av(
                                "names",
                                "Return a list of the names of all existing images.",
                                "image names",
                            ),
                            _av(
                                "type",
                                "Return the type of the image (e.g. photo or bitmap).",
                                "image type name",
                            ),
                            _av(
                                "types",
                                "Return a list of the valid image types.",
                                "image types",
                            ),
                            _av(
                                "width",
                                "Return the width of the image in pixels.",
                                "image width name",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "create": SubCommand(
                    name="create",
                    arity=Arity(1),
                    detail="Create a new image of the given type (photo or bitmap).",
                    synopsis="image create type ?name? ?option value ...?",
                ),
                "delete": SubCommand(
                    name="delete",
                    arity=Arity(0),
                    detail="Delete one or more images by name.",
                    synopsis="image delete ?name name ...?",
                ),
                "height": SubCommand(
                    name="height",
                    arity=Arity(1, 1),
                    detail="Return the height of the image in pixels.",
                    synopsis="image height name",
                ),
                "inuse": SubCommand(
                    name="inuse",
                    arity=Arity(1, 1),
                    detail="Return whether the image is in use by any widgets.",
                    synopsis="image inuse name",
                ),
                "names": SubCommand(
                    name="names",
                    arity=Arity(0, 0),
                    detail="Return a list of the names of all existing images.",
                    synopsis="image names",
                ),
                "type": SubCommand(
                    name="type",
                    arity=Arity(1, 1),
                    detail="Return the type of the image (e.g. photo or bitmap).",
                    synopsis="image type name",
                ),
                "types": SubCommand(
                    name="types",
                    arity=Arity(0, 0),
                    detail="Return a list of the valid image types.",
                    synopsis="image types",
                ),
                "width": SubCommand(
                    name="width",
                    arity=Arity(1, 1),
                    detail="Return the width of the image in pixels.",
                    synopsis="image width name",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
