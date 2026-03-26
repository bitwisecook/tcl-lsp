#!/usr/bin/env python3
"""Profile the semantic tokens pipeline directly (no LSP transport).

Calls the server-side functions with cProfile to identify hotspots.

Usage::

    python3 scripts/profile_semantic_tokens.py <file.tcl>
    python3 scripts/profile_semantic_tokens.py --top 30 <file.tcl>
    python3 scripts/profile_semantic_tokens.py --phase tokens <file.tcl>
    python3 scripts/profile_semantic_tokens.py --dump profile.prof <file.tcl>
"""

from __future__ import annotations

import argparse
import cProfile
import pstats
import sys
import time
from io import StringIO
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description="Profile semantic tokens pipeline")
    parser.add_argument("file", help="Tcl file to profile")
    parser.add_argument("--top", type=int, default=40, help="Top N functions to show")
    parser.add_argument(
        "--phase",
        choices=["all", "update", "tokens", "chunk_caches", "tokenise", "compile", "analyse"],
        default="all",
        help="Which phase to profile (default: all)",
    )
    parser.add_argument("--dump", help="Dump raw profile to file for pstats/snakeviz")
    parser.add_argument("--cumulative", action="store_true", help="Sort by cumulative time")
    args = parser.parse_args()

    abs_path = Path(args.file).resolve()
    if not abs_path.is_file():
        print(f"File not found: {abs_path}", file=sys.stderr)
        sys.exit(1)

    with open(abs_path, encoding="utf-8", errors="replace") as f:
        source = f.read()

    n_lines = source.count("\n") + 1
    print(f"File: {abs_path.name} ({n_lines} lines, {len(source)} bytes)")

    # Import server-side modules.
    from core.analysis.analyser import analyse
    from core.compiler.compilation_unit import compile_source
    from core.parsing.command_segmenter import segment_top_level_chunks
    from core.parsing.lexer import TclLexer
    from lsp.features.semantic_tokens import semantic_tokens_full
    from lsp.workspace.document_state import DocumentState

    if args.phase in ("all", "update"):
        print(f"\n{'=' * 60}")
        print("Phase: Full DocumentState.update() pipeline")
        print(f"{'=' * 60}")

        prof = cProfile.Profile()
        state = DocumentState(uri=f"file://{abs_path}")

        prof.enable()
        t0 = time.perf_counter()
        state.update(source, version=1)
        t1 = time.perf_counter()
        prof.disable()

        print(f"Total: {(t1 - t0) * 1000:.0f}ms")
        _print_stats(prof, args.top, args.cumulative)
        if args.dump:
            prof.dump_stats(f"{args.dump}.update")

        # Sub-phase profiling
        if args.phase == "all":
            _profile_sub_phases(source, abs_path, args)

    if args.phase == "tokens":
        print(f"\n{'=' * 60}")
        print("Phase: semantic_tokens_full() only")
        print(f"{'=' * 60}")

        # Run update first to get analysis
        state = DocumentState(uri=f"file://{abs_path}")
        state.update(source, version=1)

        prof = cProfile.Profile()
        prof.enable()
        t0 = time.perf_counter()
        data = semantic_tokens_full(source, analysis=state.analysis)
        t1 = time.perf_counter()
        prof.disable()

        n_tokens = len(data) // 5
        print(f"Total: {(t1 - t0) * 1000:.0f}ms ({n_tokens} tokens)")
        _print_stats(prof, args.top, args.cumulative)
        if args.dump:
            prof.dump_stats(f"{args.dump}.tokens")

    if args.phase == "chunk_caches":
        print(f"\n{'=' * 60}")
        print("Phase: _build_full_chunk_caches() only")
        print(f"{'=' * 60}")

        from core.analysis.analyser import Analyser

        state = DocumentState(uri=f"file://{abs_path}")
        # Do the update without chunk caches, then profile chunk cache building
        chunks = segment_top_level_chunks(source)
        state.chunks = chunks
        state.source = source
        state.tokens = TclLexer(source).tokenise_all()
        try:
            state.compilation_unit = compile_source(source)
        except Exception:
            state.compilation_unit = None
        chunk_commands = [list(chunk.commands) for chunk in chunks]
        analyser = Analyser()
        state.analysis, chunk_snapshots = analyser.analyse_chunked(
            source, chunk_commands, cu=state.compilation_unit
        )

        prof = cProfile.Profile()
        prof.enable()
        t0 = time.perf_counter()
        state._build_full_chunk_caches(source, chunks, chunk_snapshots)
        t1 = time.perf_counter()
        prof.disable()

        print(f"Total: {(t1 - t0) * 1000:.0f}ms ({len(chunks)} chunks)")
        _print_stats(prof, args.top, args.cumulative)
        if args.dump:
            prof.dump_stats(f"{args.dump}.chunk_caches")

    if args.phase == "tokenise":
        print(f"\n{'=' * 60}")
        print("Phase: TclLexer.tokenise_all() only")
        print(f"{'=' * 60}")

        prof = cProfile.Profile()
        prof.enable()
        t0 = time.perf_counter()
        tokens = TclLexer(source).tokenise_all()
        t1 = time.perf_counter()
        prof.disable()

        print(f"Total: {(t1 - t0) * 1000:.0f}ms ({len(tokens)} tokens)")
        _print_stats(prof, args.top, args.cumulative)

    if args.phase == "compile":
        print(f"\n{'=' * 60}")
        print("Phase: compile_source() only")
        print(f"{'=' * 60}")

        prof = cProfile.Profile()
        prof.enable()
        t0 = time.perf_counter()
        try:
            cu = compile_source(source)
        except Exception as e:
            print(f"Compilation failed: {e}")
            cu = None
        t1 = time.perf_counter()
        prof.disable()

        n_procs = len(cu.procedures) if cu else 0
        print(f"Total: {(t1 - t0) * 1000:.0f}ms ({n_procs} procs)")
        _print_stats(prof, args.top, args.cumulative)

    if args.phase == "analyse":
        print(f"\n{'=' * 60}")
        print("Phase: analyse() only")
        print(f"{'=' * 60}")

        try:
            cu = compile_source(source)
        except Exception:
            cu = None

        prof = cProfile.Profile()
        prof.enable()
        t0 = time.perf_counter()
        analyse(source, cu=cu)
        t1 = time.perf_counter()
        prof.disable()

        print(f"Total: {(t1 - t0) * 1000:.0f}ms")
        _print_stats(prof, args.top, args.cumulative)


def _profile_sub_phases(source: str, abs_path: Path, args: argparse.Namespace) -> None:
    """Profile individual sub-phases of the update pipeline."""
    from core.analysis.analyser import Analyser, analyse
    from core.compiler.compilation_unit import compile_source
    from core.compiler.lowering import lower_commands_to_ir
    from core.parsing.command_segmenter import segment_top_level_chunks
    from core.parsing.lexer import TclLexer
    from lsp.features.semantic_tokens import semantic_tokens_full

    print(f"\n{'=' * 60}")
    print("Sub-phase breakdown")
    print(f"{'=' * 60}")

    # Segment
    t0 = time.perf_counter()
    chunks = segment_top_level_chunks(source)
    t1 = time.perf_counter()
    print(f"  segment_top_level_chunks: {(t1 - t0) * 1000:.0f}ms ({len(chunks)} chunks)")

    # Tokenise
    t0 = time.perf_counter()
    tokens = TclLexer(source).tokenise_all()
    t1 = time.perf_counter()
    print(f"  TclLexer.tokenise_all:   {(t1 - t0) * 1000:.0f}ms ({len(tokens)} tokens)")

    # Compile
    t0 = time.perf_counter()
    try:
        cu = compile_source(source)
    except Exception:
        cu = None
    t1 = time.perf_counter()
    n_procs = len(cu.procedures) if cu else 0
    print(f"  compile_source:          {(t1 - t0) * 1000:.0f}ms ({n_procs} procs)")

    # Analyse
    t0 = time.perf_counter()
    analysis = analyse(source, cu=cu)
    t1 = time.perf_counter()
    print(f"  analyse:                 {(t1 - t0) * 1000:.0f}ms")

    # Chunk caches (profile this more deeply since it's the bottleneck)
    print("\n  _build_full_chunk_caches sub-profile:")
    from core.common.document_buffer import DocumentBuffer

    t_total_lower = 0.0
    t_total_analyse = 0.0
    t_total_style = 0.0
    t_total_snap = 0.0

    analyser = Analyser()
    analyser._source = source
    buf = DocumentBuffer.from_source(source)
    from lsp.features.diagnostics import compute_style_diagnostics_for_range

    for i, chunk in enumerate(chunks):
        cmds = list(chunk.commands)

        t0 = time.perf_counter()
        ir_stmts, ir_procs = lower_commands_to_ir(source, cmds)
        t1 = time.perf_counter()
        t_total_lower += t1 - t0

        t0 = time.perf_counter()
        analyser._analyse_commands_inner(cmds, analyser._current_scope, source)
        t1 = time.perf_counter()
        t_total_analyse += t1 - t0

        t0 = time.perf_counter()
        analyser.snapshot()
        t1 = time.perf_counter()
        t_total_snap += t1 - t0

        t0 = time.perf_counter()
        start_line, _sc, end_line, _ec = buf.chunk_line_range(chunk.start_offset, chunk.end_offset)
        compute_style_diagnostics_for_range(source, start_line, end_line)
        t1 = time.perf_counter()
        t_total_style += t1 - t0

    print(f"    lower_commands_to_ir:  {t_total_lower * 1000:.0f}ms")
    print(f"    analyse (snapshots):   {t_total_analyse * 1000:.0f}ms")
    print(f"    analyser.snapshot():   {t_total_snap * 1000:.0f}ms")
    print(f"    style diagnostics:     {t_total_style * 1000:.0f}ms")

    # Semantic tokens
    t0 = time.perf_counter()
    data = semantic_tokens_full(source, analysis=analysis)
    t1 = time.perf_counter()
    n_tokens = len(data) // 5
    print(f"\n  semantic_tokens_full:    {(t1 - t0) * 1000:.0f}ms ({n_tokens} tokens)")


def _print_stats(prof: cProfile.Profile, top: int, cumulative: bool) -> None:
    s = StringIO()
    ps = pstats.Stats(prof, stream=s)
    ps.strip_dirs()
    sort_key = "cumulative" if cumulative else "tottime"
    ps.sort_stats(sort_key)
    ps.print_stats(top)
    print(s.getvalue())


if __name__ == "__main__":
    main()
