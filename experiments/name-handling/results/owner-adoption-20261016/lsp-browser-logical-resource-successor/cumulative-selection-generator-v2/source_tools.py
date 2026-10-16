#!/usr/bin/env python3
"""Generate a source-bound exact selection; never invoke Cargo or test binaries."""
import argparse
import copy
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path('/workspace/tcl-lsp')
BASE = Path('/workspace/naming-validation/lsp-source-path-diagnostics-repair/selection.json')
DESTINATION = '/workspace/naming-validation/lsp-readonly-cache-successor-repair2'
TARGETS = {name.replace('-', '_'): ROOT / 'rust' / name / 'src' / 'lib.rs' for name in
           ['tcl-compiler', 'tcl-registry', 'tcl-lsp-core', 'tcl-lsp-db', 'tcl-lsp-server']}
RAW_STRING = re.compile(r'(?:br|cr|r)(#{0,255})"')
CHARACTER = re.compile(r"'(?:[^'\\\n]|\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.))'")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def mask_rust(text):
    """Mask Rust comments/literals while preserving offsets and newlines."""
    out = list(text)
    i = 0
    while i < len(text):
        end = None
        if text.startswith('//', i):
            end = text.find('\n', i)
            if end < 0:
                end = len(text)
        elif text.startswith('/*', i):
            cursor, nesting = i + 2, 1
            while cursor < len(text) and nesting:
                if text.startswith('/*', cursor):
                    nesting += 1
                    cursor += 2
                elif text.startswith('*/', cursor):
                    nesting -= 1
                    cursor += 2
                else:
                    cursor += 1
            assert nesting == 0, 'unterminated source comment'
            end = cursor
        else:
            raw = RAW_STRING.match(text, i) if text[i] in 'rbc' else None
            if raw:
                closing = '"' + raw.group(1)
                close = text.find(closing, raw.end())
                assert close >= 0, 'unterminated raw string'
                end = close + len(closing)
            elif text[i] == '"':
                cursor = i + 1
                while cursor < len(text):
                    if text[cursor] == '\\':
                        cursor += 2
                    elif text[cursor] == '"':
                        cursor += 1
                        break
                    else:
                        cursor += 1
                end = cursor
            elif text[i] == "'":
                char = CHARACTER.match(text, i)
                if char:
                    end = char.end()
        if end is not None:
            for pos in range(i, end):
                if out[pos] != '\n':
                    out[pos] = ' '
            i = end
        else:
            i += 1
    return ''.join(out)


class SourceIndex:
    def __init__(self):
        self.cache = {}

    def load(self, path):
        if path not in self.cache:
            text = path.read_text()
            masked = mask_rust(text)
            depths, stack, pairs = [], [], {}
            for pos, char in enumerate(masked):
                depths.append(len(stack))
                if char == '{':
                    stack.append(pos)
                elif char == '}':
                    assert stack, (path, pos)
                    pairs[stack.pop()] = pos
            assert not stack, path
            self.cache[path] = text, masked, depths, pairs
        return self.cache[path]

    def resolve(self, target, selector, purpose):
        parts = selector.split('::')
        path = TARGETS[target]
        start, end, level = 0, len(path.read_text()), 0
        module_dir = path.parent
        route = []
        for part in parts[:-1]:
            text, masked, depths, pairs = self.load(path)
            matches = [m for m in re.finditer(r'\bmod\s+' + re.escape(part) + r'\s*([;{])', masked, re.M)
                       if start <= m.start() < end and depths[m.start()] == level]
            assert len(matches) == 1, (target, selector, path, part, len(matches))
            match = matches[0]
            route.append({'source_path': str(path.relative_to(ROOT)),
                          'source_sha256': digest(path),
                          'source_line': text.count('\n', 0, match.start()) + 1,
                          'module': part})
            delimiter = match.end() - 1
            if match.group(1) == '{':
                start, end, level = delimiter + 1, pairs[delimiter], level + 1
                module_dir = module_dir / part
            else:
                prefix = text[max(start, match.start() - 1500):match.start()]
                prefix = re.sub(r'pub(?:\([^)]*\))?\s*$', '', prefix)
                tail = re.search(r'((?:\s*#\[[^\]]*\]\s*)*)$', prefix).group(1)
                attr = re.search(r'#\[path\s*=\s*"([^"]+)"\s*\]', tail)
                candidates = [module_dir / attr.group(1)] if attr else [module_dir / (part + '.rs'), module_dir / part / 'mod.rs']
                candidates = [candidate for candidate in candidates if candidate.is_file()]
                assert len(candidates) == 1, (selector, part, candidates)
                path = candidates[0].resolve()
                module_dir = path.parent if path.name == 'mod.rs' else path.parent / path.stem
                start, end, level = 0, len(path.read_text()), 0
        text, masked, depths, pairs = self.load(path)
        matches = [m for m in re.finditer(r'\b(?:async\s+)?fn\s+' + re.escape(parts[-1]) + r'\s*\(', masked)
                   if start <= m.start() < end and depths[m.start()] == level]
        assert len(matches) == 1, (target, selector, path, len(matches))
        match = matches[0]
        assert re.search(r'#\[(?:test|tokio::test|test_case)[^\]]*\]', text[max(start, match.start() - 200):match.start()]), (selector, 'missing actual test attribute')
        return {'target': target, 'selector': selector, 'source_path': str(path.relative_to(ROOT)),
                'source_line': text.count('\n', 0, match.start()) + 1,
                'source_sha256': digest(path), 'purpose': purpose, 'module_route': route}


import sys
def attached_attributes(text, start, position):
    prefix = text[max(start, position - 1200):position]
    found = re.search(r'((?:\s*#\[[^\]]*\]\s*)+)$', prefix)
    return found.group(1).strip() if found else ''


def native_linux_cfg(expression):
    """Evaluate the exact test/unix/windows/target_os forms in this selection."""
    assert sys.platform == 'linux', 'native Linux source eligibility requires actual Linux host'
    tokens = re.findall(r'"[^"\n]*"|[A-Za-z_][A-Za-z_0-9]*|[(),=]', expression)
    position = 0
    def parse():
        nonlocal position
        name = tokens[position]; position += 1
        if name in ['all','any','not']:
            assert tokens[position]=='('; position += 1
            args=[]
            while tokens[position]!=')':
                args.append(parse())
                if tokens[position]==',':position += 1
                else:assert tokens[position]==')'
            position += 1
            if name=='all':return all(args)
            if name=='any':return any(args)
            assert len(args)==1
            return not args[0]
        if name=='target_os':
            assert tokens[position]=='='; position += 1
            value=tokens[position]; position += 1
            assert value in ['"wasi"','"linux"'], ('unsupported target_os requires source review',expression)
            return value=='"linux"'
        assert name in ['test','unix','windows'], ('unsupported cfg requires source review',expression)
        return name in ['test','unix']
    result=parse()
    assert position==len(tokens)
    return result


def source_eligibility(index, row):
    """Current declaration/body/module hashes plus explicit current host cfg."""
    path = ROOT / row['source_path']
    text, masked, depths, pairs = index.load(path)
    assert digest(path) == hashlib.sha256(text.encode()).hexdigest(), ('source changed during capture', str(path))
    start = sum(len(line) for line in text.splitlines(keepends=True)[:row['source_line']-1])
    match = re.search(r'\b(?:async\s+)?fn\s+' + re.escape(row['selector'].split('::')[-1]) + r'\s*\(', masked[start:])
    assert match and '\n' not in masked[start:start+match.start()], ('wrong current test source line', row)
    position = start + match.start()
    opening = masked.index('{', position)
    ending = pairs[opening]+1
    attrs = attached_attributes(text, 0, position)
    cfgs = re.findall(r'#\[cfg\((.*?)\)\]', attrs, re.S)
    routes = []
    for route in row['module_route']:
        modpath = ROOT / route['source_path']
        modtext, _, _, _ = index.load(modpath)
        offset = sum(len(line) for line in modtext.splitlines(keepends=True)[:route['source_line']-1])
        modpos = modtext.index('mod ', offset)
        attributes = attached_attributes(modtext, 0, modpos)
        modcfgs = re.findall(r'#\[cfg\((.*?)\)\]', attributes, re.S)
        cfgs.extend(modcfgs)
        routes.append(dict(route, attributes=attributes, cfg_expressions=modcfgs))
    assert all(native_linux_cfg(cfg) for cfg in cfgs), ('current native Linux source excludes selected test', row['selector'], cfgs)
    return dict(row, function_attributes=attrs, cfg_expressions=cfgs,
                current_native_linux_source_eligible=True,
                actual_test_function_sha256=hashlib.sha256(text[position:ending].encode()).hexdigest(),
                checked_module_route=routes,
                eligibility_scope='Current actual source declaration and recognised cfg only; actual frozen listing singleton remains required before execution.')

