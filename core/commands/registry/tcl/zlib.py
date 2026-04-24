"""zlib -- Data compression / decompression primitives (Tcl 8.6+)."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page zlib.n"


@register
class ZlibCommand(CommandDef):
    name = "zlib"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="zlib",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Compression / decompression using zlib.",
                synopsis=(
                    "zlib compress data ?level?",
                    "zlib decompress data ?bufferSize?",
                    "zlib deflate data ?level?",
                    "zlib inflate data ?bufferSize?",
                    "zlib gzip data ?-level level? ?-header header?",
                    "zlib gunzip data ?-buffersize n? ?-headerVar varname?",
                    "zlib crc32 data ?initValue?",
                    "zlib adler32 data ?initValue?",
                    "zlib stream mode ?level?",
                    "zlib push mode channel ?options?",
                ),
                snippet=(
                    "Compress / decompress data, compute CRC32 / Adler-32 "
                    "checksums, or attach a compression filter to a "
                    "channel.  Not yet implemented in the WASM runtime — "
                    "traps with ``unsupported command: zlib``."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="zlib subcommand ?args ...?",
                ),
            ),
            validation=ValidationSpec(
                # Sub-command dispatch at runtime; the min/max span covers
                # every sub-command (``compress data`` → 2,
                # ``push mode chan ?opts?`` → variadic).
                arity=Arity(1),
            ),
            return_type=TclType.STRING,
        )
