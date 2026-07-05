# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tcl package resolution via pkgIndex.tcl files.

Parses ``pkgIndex.tcl`` files to build a mapping from package names to the
source files that provide them.  The resolver is used to satisfy
``package require`` statements encountered during analysis.
"""

from __future__ import annotations

import logging
import os
import re
from dataclasses import dataclass
from pathlib import Path

from compiler.parsing.green_tree import tokenise
from shared.tokens import Token, TokenType

log = logging.getLogger(__name__)

# A pkgIndex.tcl version token's accepted shape: digits/dots with an optional
# alpha/beta suffix (e.g. ``1.0``, ``2.1``, ``1.0b1``).  This is a narrow
# *format* gate on an already-tokenised word — not Tcl structural parsing — so
# it stays a regex.  Words that fail it are skipped, matching the old behaviour
# where ``package ifneeded`` lines with an unconventional version were ignored.
_VERSION_RE = re.compile(r"[\d.]+(?:[ab]\d+)?")


def _walk_command_words(text: str) -> list[list[list[Token]]]:
    """Tokenise *text* and group tokens into commands of words.

    Each command is a list of words; each word is the list of adjacent
    non-separator tokens between ``SEP`` / ``EOL`` boundaries (so ``a$b[c]`` is
    one word of three tokens).  ``{*}`` expand markers are treated as word
    boundaries, matching :func:`parse_single_command`.
    """
    tokens, _ = tokenise(text, 0, 0, 0)
    commands: list[list[list[Token]]] = []
    words: list[list[Token]] = []
    word: list[Token] = []

    def end_word() -> None:
        nonlocal word
        if word:
            words.append(word)
        word = []

    def end_command() -> None:
        nonlocal words
        end_word()
        if words:
            commands.append(words)
        words = []

    for tok in tokens:
        if tok.type is TokenType.COMMENT:
            continue
        if tok.type is TokenType.EOL:
            end_command()
            continue
        if tok.type in (TokenType.SEP, TokenType.EXPAND):
            end_word()
            continue
        word.append(tok)

    end_command()
    return commands


def _word_raw(text: str, word: list[Token]) -> str:
    """Return the verbatim source slice spanning *word* within *text*."""
    return text[word[0].start.offset : word[-1].end.offset + 1]


def _word_unwrap(text: str, word: list[Token]) -> str | None:
    """Return the inner script text of *word* if it is a single wrapper.

    Handles the three wrappers a pkgIndex ``ifneeded`` body uses:

    * ``[...]`` command substitution — a single ``CMD`` token whose ``.text`` is
      already the inner script.
    * ``{...}`` braced literal — a single ``STR`` token whose ``.text`` is the
      brace-stripped body.
    * ``"..."`` quoted word — the run of in-quote tokens; the inner text is the
      source slice from just after the opening quote through the last content
      token (the closing quote is excluded by construction).

    Returns ``None`` when *word* is not one of these (so the caller does not
    descend into a bare word) and to break the recursion when unwrapping a
    quoted word would reproduce it.  Using token kinds rather than raw bracket
    matching avoids the ``[...]`` span quirk where the ``CMD`` token's end
    excludes the outer closing bracket.
    """
    if len(word) == 1:
        tok = word[0]
        if tok.type is TokenType.CMD or tok.type is TokenType.STR:
            return tok.text
    open_off = word[0].start.offset
    if word[0].in_quote and open_off < len(text) and text[open_off] == '"':
        # A double-quoted word.  The closing quote is not consistently attached
        # to a token (it may or may not appear as a trailing zero-width token),
        # so locate it by a backslash-aware forward scan from just after the
        # opening quote; the inner script is the span between the two quotes.
        i = open_off + 1
        while i < len(text):
            ch = text[i]
            if ch == "\\":
                i += 2
                continue
            if ch == '"':
                return text[open_off + 1 : i]
            i += 1
        return text[open_off + 1 :]
    return None


def _source_filename(text: str, arg: list[Token]) -> str | None:
    """Extract the package-relative filename from a ``source`` argument word.

    Accepts the two structural forms the old regexes matched, but via the
    tokeniser:

    * ``[file join $dir X ...]`` — the words after ``$dir`` joined with a single
      space (mirroring the old ``[^\\]]+`` capture), with surrounding quotes
      stripped.
    * ``$dir/X`` — the path tail after a bare ``$dir/`` prefix.

    Returns ``None`` when *arg* is neither form.
    """
    raw = _word_raw(text, arg)

    # ``[file join $dir X]`` — a single command-substitution word.
    if len(arg) == 1 and arg[0].type is TokenType.CMD:
        inner = arg[0].text
        cmds = _walk_command_words(inner)
        if len(cmds) != 1:
            return None
        words = cmds[0]
        if len(words) < 4:  # file join $dir <tail...>
            return None
        if _word_raw(inner, words[0]) != "file" or _word_raw(inner, words[1]) != "join":
            return None
        if _word_raw(inner, words[2]) not in ("$dir", "${dir}"):
            return None
        tail = " ".join(_word_raw(inner, w) for w in words[3:])
        return tail.strip().strip('"')

    # ``$dir/X`` — bare variable substitution immediately followed by ``/tail``.
    for prefix in ("$dir/", "${dir}/"):
        if raw.startswith(prefix):
            return raw[len(prefix) :].strip().strip('"')
    return None


def _collect_source_targets(
    text: str, words_list: list[list[list[Token]]], pkg_dir: str, files: list[str]
) -> None:
    """Walk *words_list* (commands of *text*) collecting ``source`` targets.

    For each command, adjacent ``source <arg>`` word pairs are checked against
    the two structural file forms; matches that resolve to a real file under
    *pkg_dir* are appended to *files*.  Each word that is itself a wrapper
    (``[...]`` command substitution, ``{...}`` braced body, or ``"..."`` quoted
    body) is descended into recursively, so ``[list source [file join $dir X]]``
    and ``"source $dir/X"`` are reached without text-level pattern matching.
    """
    for words in words_list:
        for i, word in enumerate(words):
            if _word_raw(text, word) == "source" and i + 1 < len(words):
                filename = _source_filename(text, words[i + 1])
                if filename is not None:
                    full_path = os.path.join(pkg_dir, filename)
                    if os.path.isfile(full_path):
                        files.append(full_path)
        for word in words:
            inner = _word_unwrap(text, word)
            if inner is not None:
                _collect_source_targets(inner, _walk_command_words(inner), pkg_dir, files)


def _iter_pkg_ifneeded(content: str) -> list[tuple[str, str, list[list[Token]]]]:
    """Yield ``(name, version, body_words)`` for each ``package ifneeded`` command.

    Replaces the old ``_PKG_IFNEEDED_RE`` line scan with token-based command
    parsing: the file is tokenised, every command is walked, and any command
    whose first two words are ``package ifneeded`` is taken as a declaration.
    The version word must still match the conventional version *format* (a
    narrow gate, not Tcl structure); declarations with an unconventional version
    are skipped, matching the old regex.  *body_words* is the list of remaining
    words (the ``ifneeded`` body, anchored in *content*) for source extraction.
    """
    results: list[tuple[str, str, list[list[Token]]]] = []
    for words in _walk_command_words(content):
        if len(words) < 5:
            continue
        if _word_raw(content, words[0]) != "package":
            continue
        if _word_raw(content, words[1]) != "ifneeded":
            continue
        name = _word_raw(content, words[2])
        version = _word_raw(content, words[3])
        if not _VERSION_RE.fullmatch(version):
            continue
        results.append((name, version, words[4:]))
    return results


@dataclass
class PackageInfo:
    """Metadata for a discovered Tcl package."""

    name: str
    version: str
    source_files: list[str]  # absolute paths to implementation files
    pkg_index_path: str  # path to the pkgIndex.tcl that declared this


class PackageResolver:
    """Resolves ``package require`` to source files via pkgIndex.tcl scanning."""

    def __init__(self) -> None:
        self._packages: dict[str, list[PackageInfo]] = {}  # name -> versions
        self._auto_index: dict[str, list[str]] = {}  # proc_name -> [abs paths]
        self._search_paths: list[str] = []
        self._scanned_paths: set[str] = set()
        self._scanned: bool = False

    def configure(self, search_paths: list[str]) -> None:
        """Replace the search-path list and invalidate the scan cache.

        Workspace roots should appear first so that a workspace-local
        copy of a package wins over any copy provided by an installed
        library path — matching Tcl's own ``auto_path`` order, where the
        first ``pkgIndex.tcl`` that satisfies a ``package require`` is
        the one that takes effect.
        """
        self._search_paths = list(search_paths)
        self._scanned_paths.clear()
        self._scanned = False

    def add_search_paths(self, paths: list[str], *, prepend: bool = False) -> None:
        """Incorporate additional search paths without discarding the scan cache.

        Used to honour a document's own ``lappend auto_path`` declarations:
        the new paths are merged in (deduplicated) and scanned immediately
        for packages and tclIndex entries.  Existing ``_packages`` entries
        are preserved, so the first provider already discovered keeps
        priority.  Pass ``prepend=True`` to place the new entries at the
        front of the effective order (for workspace-wins semantics).
        """
        seen_new: set[str] = set()
        new_paths: list[str] = []
        for path in paths:
            if not path:
                continue
            expanded = os.path.expanduser(path)
            abs_path = os.path.abspath(expanded)
            if abs_path in self._scanned_paths or abs_path in seen_new:
                continue
            seen_new.add(abs_path)
            new_paths.append(abs_path)
        if not new_paths:
            return
        if prepend:
            self._search_paths = new_paths + [
                p
                for p in self._search_paths
                if os.path.abspath(os.path.expanduser(p)) not in seen_new
            ]
        else:
            existing = {os.path.abspath(os.path.expanduser(p)) for p in self._search_paths}
            self._search_paths.extend(p for p in new_paths if p not in existing)
        for path in new_paths:
            self._scan_single_path(path)

    def scan_packages(self) -> None:
        """Walk search paths looking for pkgIndex.tcl and tclIndex files.

        Paths are visited in ``_search_paths`` order and the first
        ``package ifneeded`` entry for a given name wins — later
        providers append to the version list but do not displace the
        head.  This mirrors Tcl's own ``auto_path`` semantics.
        """
        self._packages.clear()
        self._auto_index.clear()
        self._scanned_paths.clear()
        for search_path in self._search_paths:
            expanded = os.path.expanduser(search_path)
            if not os.path.isdir(expanded):
                log.warning("PackageResolver: directory not found: %s", search_path)
                continue
            self._scan_single_path(os.path.abspath(expanded))
        self._scanned = True
        log.info(
            "Package scan: found %d packages, %d auto-index procs in %d search paths",
            len(self._packages),
            len(self._auto_index),
            len(self._search_paths),
        )

    def _scan_single_path(self, abs_path: str) -> None:
        """Walk *abs_path* recording pkgIndex.tcl and tclIndex entries."""
        if abs_path in self._scanned_paths:
            return
        # Don't mark a non-existent directory as scanned — a transient
        # mount or a path created later should still get picked up on
        # a subsequent call.
        if not os.path.isdir(abs_path):
            return
        self._scanned_paths.add(abs_path)
        for root, _dirs, files in os.walk(abs_path):
            if "pkgIndex.tcl" in files:
                pkg_index_path = os.path.join(root, "pkgIndex.tcl")
                self._parse_pkg_index(pkg_index_path, root)
            files_lower = {f.lower(): f for f in files}
            if "tclindex" in files_lower:
                tcl_index_path = os.path.join(root, files_lower["tclindex"])
                self._parse_auto_index(tcl_index_path)

    def resolve(self, package_name: str, version: str | None = None) -> list[str]:
        """Resolve a package name to its source file paths."""
        if not self._scanned:
            self.scan_packages()

        infos = self._packages.get(package_name, [])
        if not infos:
            return []

        if version:
            for info in infos:
                if info.version == version or info.version.startswith(version):
                    return list(info.source_files)
            # Specific version requested but not found
            return []

        # No version constraint -- return files from the first entry
        return list(infos[0].source_files)

    def all_package_names(self) -> list[str]:
        if not self._scanned:
            self.scan_packages()
        return list(self._packages.keys())

    def _parse_pkg_index(self, pkg_index_path: str, pkg_dir: str) -> None:
        """Parse a pkgIndex.tcl file."""
        try:
            content = Path(pkg_index_path).read_text(
                encoding="utf-8",
                errors="replace",
            )
        except Exception:
            log.debug(
                "PackageResolver: failed to read %s",
                pkg_index_path,
                exc_info=True,
            )
            return

        for name, version, body_words in _iter_pkg_ifneeded(content):
            source_files = self._extract_source_files(content, body_words, pkg_dir)

            if source_files:
                info = PackageInfo(
                    name=name,
                    version=version,
                    source_files=source_files,
                    pkg_index_path=pkg_index_path,
                )
                self._packages.setdefault(name, []).append(info)

    def _extract_source_files(
        self, content: str, body_words: list[list[Token]], pkg_dir: str
    ) -> list[str]:
        """Extract source file paths from a pkgIndex.tcl ``ifneeded`` body.

        *body_words* are the tokenised words of a ``package ifneeded`` body
        (anchored in *content*) — typically a ``[list source [file join $dir X]]``
        or ``"source $dir/X"`` wrapper.  It is walked through the tokeniser (no
        structural regexes): every nested command is examined and any
        ``source [file join $dir X]`` or ``source $dir/X`` form is collected,
        descending through ``[...]`` command substitutions and ``{...}`` /
        ``"..."`` wrappers.
        """
        files: list[str] = []
        _collect_source_targets(content, [body_words], pkg_dir, files)

        # Fallback: if no explicit source lines, look for .tcl files in the dir
        if not files:
            try:
                for f in os.listdir(pkg_dir):
                    if f.endswith(".tcl") and f != "pkgIndex.tcl":
                        files.append(os.path.join(pkg_dir, f))
            except OSError:
                pass

        return files

    def _parse_auto_index(self, tcl_index_path: str) -> None:
        """Parse a tclIndex file and register proc->file mappings."""
        from .auto_index import parse_tcl_index

        for entry in parse_tcl_index(tcl_index_path):
            self._auto_index.setdefault(entry.proc_name, []).append(entry.source_file)

    def resolve_auto_proc(self, proc_name: str) -> list[str]:
        """Resolve an auto-loaded proc name to its source file paths."""
        if not self._scanned:
            self.scan_packages()
        return list(self._auto_index.get(proc_name, []))

    def all_auto_proc_names(self) -> list[str]:
        """Return all known auto-loadable proc names from tclIndex files."""
        if not self._scanned:
            self.scan_packages()
        return list(self._auto_index.keys())
