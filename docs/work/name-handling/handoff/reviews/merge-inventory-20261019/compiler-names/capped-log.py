#!/usr/bin/env python3
"""Drain a test process while retaining bounded diagnostics and result lines."""
import re
import sys
pattern = re.compile(rb'test |test result:|panicked|FAILED|assert|left|right|expected|lost|ZZ_|error|warning|Compiling|Finished|Running|^ +["|=^]')
remaining = 20_000_000
with open(sys.argv[1], 'wb') as output:
    for line in sys.stdin.buffer:
        if pattern.search(line) and remaining:
            kept = line[:min(remaining, 4000)]
            output.write(kept)
            output.flush()
            remaining -= len(kept)
