// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// explorer-core.js — shared rendering logic for the Tcl Compiler Explorer.
//
// Loaded by:
//   - explorer/static/index.html  (standalone web app, via <script src>)
//   - editors/vscode/src/compilerExplorerHtml.ts  (VS Code webview, inlined)
//   - editors/jetbrains/ (extracted from VS Code build)
//
// Consumers MUST define these globals before this script runs:
//   data, compiledSource, compiledDialect, $, $$
//
// Consumers MUST define these hook functions (before or after this script):
//   getSource()                      — returns the current source text
//   setupHoverHighlighting(el)       — wires click/hover on [data-start] elements
//   buildOptDiffView()               — returns HTML for the optimiser diff
//   setupOptDiffHover(pane)          — wires hover/click on diff groups
//   setupOptItemDiffScroll(pane)     — wires opt-item click to scroll diff
//   renderShimmer()                  — renders the shimmer pane
//   renderIrulesFlow()               — renders the iRules flow pane
//   renderAll()                      — calls all render functions
//   updateBadges()                   — updates tab badge counts

// Utility
function esc(s){if(s==null)return'';return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;').replace(/'/g,'&#39;');}
var isMac = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent || '');

// Status light
var statusLight = document.getElementById('statusLight');
var STATUS_TITLES = { loading: 'Loading...', dirty: 'Out of sync', compiling: 'Compiling...', synced: 'In sync' };
function setStatus(state) {
  statusLight.className = 'status-light ' + state;
  statusLight.title = STATUS_TITLES[state] || state;
}

function showError(msg, tb) {
  var pane = $('#pane-ir');
  pane.innerHTML = '<div class="error-box">' + esc(msg) + (tb ? '\n\n' + esc(tb) : '') + '</div>';
}

// Hover highlighting helpers
function sourceRangeAttrs(range) {
  if (!range) return '';
  return ' data-start="' + range.startOffset + '" data-end="' + range.endOffset + '"';
}
function spanLabel(range) {
  if (!range) return '';
  return (range.startLine + 1) + ':' + (range.startCol + 1);
}
var currentHighlighted = null;

// Stats
function renderStats() {
  var s = data.stats;
  var parts = [
    'procs <span class="stat-value">' + s.procedures + '</span>',
    'blocks <span class="stat-value">' + s.blocks + '</span>',
    'dead stores <span class="stat-value">' + s.deadStores + '</span>',
    'unreachable <span class="stat-value">' + s.unreachableBlocks + '</span>',
    'rewrites <span class="stat-value">' + s.rewrites + '</span>',
    'shimmer <span class="stat-value">' + s.shimmerWarnings + '</span>',
  ];
  if (s.dataflowDefs !== undefined) {
    parts.push('defs <span class="stat-value">' + s.dataflowDefs + '</span>');
    parts.push('aliases <span class="stat-value">' + (s.dataflowAliases || 0) + '</span>');
  }
  if (s.gvnWarnings) parts.push('gvn <span class="stat-value">' + s.gvnWarnings + '</span>');
  if (s.taintWarnings) parts.push('taint <span class="stat-value">' + s.taintWarnings + '</span>');
  if (s.irulesFlowWarnings) parts.push('irules <span class="stat-value">' + s.irulesFlowWarnings + '</span>');
  $('#stats').innerHTML = parts.join(' &middot; ');
}

// IR rendering
function renderIR() {
  var pane = $('#pane-ir');
  var optAvail = !!data.irOptimised;
  var mode = optAvail ? (structOptState['pane-ir'] || 'off') : 'off';
  var toolbar = renderOptToolbar('pane-ir', optAvail);
  if (mode === 'diff') {
    pane.innerHTML = toolbar + renderTextDiff(irToLines(data.ir), irToLines(data.irOptimised));
    setupOptToolbar(pane, 'pane-ir', renderIR);
    return;
  }
  var ir = mode === 'on' ? data.irOptimised : data.ir;
  var html = toolbar + '<div class="section-header">top-level</div><div class="ir-tree">';
  html += renderIRNodes(ir.topLevel);
  html += '</div>';
  var procs = ir.procedures;
  if (Object.keys(procs).length) {
    html += '<div class="section-header">procedures</div>';
    for (var _e of Object.entries(procs)) {
      var name = _e[0], proc = _e[1];
      var params = proc.params.length ? ' {' + proc.params.join(' ') + '}' : ' {}';
      html += '<div class="section-header" style="font-size:12px; color:var(--cyan); border:none; margin:8px 0 2px">' + esc(name) + esc(params) + ' <span style="color:var(--text-dim); font-size:10px">[' + spanLabel(proc.range) + ']</span></div>';
      html += '<div class="ir-tree">' + renderIRNodes(proc.body) + '</div>';
    }
  }
  pane.innerHTML = html;
  setupHoverHighlighting(pane);
  setupIRToggle(pane);
  setupOptToolbar(pane, 'pane-ir', renderIR);
}
function renderIRNodes(nodes) {
  if (!nodes || !nodes.length) return '<div style="color:var(--text-dim); margin-left:16px; font-size:11px">(empty)</div>';
  var html = '';
  for (var node of nodes) {
    var hasChildren = node.children && node.children.length;
    html += '<div class="ir-node">';
    html += '<div class="ir-node-header"' + sourceRangeAttrs(node.range) + '>';
    html += '<span class="toggle">' + (hasChildren ? '\u25b8' : ' ') + '</span>';
    html += '<span class="summary ' + node.colorClass + '">' + esc(node.summary) + '</span>';
    html += '<span class="span-label">[' + spanLabel(node.range) + ']</span></div>';
    if (hasChildren) {
      html += '<div class="ir-node-children collapsed">';
      for (var child of node.children) {
        html += '<div class="ir-child-label">' + esc(child.label) + '</div>';
        html += renderIRNodes(child.body);
      }
      html += '</div>';
    }
    html += '</div>';
  }
  return html;
}
function setupIRToggle(container) {
  container.addEventListener('click', function(e) {
    var header = e.target.closest('.ir-node-header');
    if (!header) return;
    var node = header.closest('.ir-node');
    var children = node.querySelector('.ir-node-children');
    if (!children) return;
    var toggle = header.querySelector('.toggle');
    children.classList.toggle('collapsed');
    toggle.textContent = children.classList.contains('collapsed') ? '\u25b8' : '\u25be';
  });
}

// CFG (pre-SSA)
function renderCfgPre() {
  var pane = $('#pane-cfg-pre');
  var optAvail = !!data.cfgPreSsaOptimised;
  var mode = optAvail ? (structOptState['pane-cfg-pre'] || 'off') : 'off';
  var toolbar = renderOptToolbar('pane-cfg-pre', optAvail);
  if (mode === 'diff') {
    pane.innerHTML = toolbar + renderTextDiff(cfgToLines(data.cfgPreSsa), cfgToLines(data.cfgPreSsaOptimised));
    setupOptToolbar(pane, 'pane-cfg-pre', renderCfgPre);
    return;
  }
  var funcs = mode === 'on' ? data.cfgPreSsaOptimised : data.cfgPreSsa;
  var html = toolbar;
  for (var func of funcs) {
    html += '<div class="cfg-function cfg-edges-container" data-func="' + esc(func.name) + '">';
    html += '<div class="cfg-func-header">' + esc(func.name) + ' <span style="color:var(--text-dim); font-size:11px">entry=' + esc(func.entry) + ' blocks=' + func.blockCount + '</span></div>';
    for (var block of func.blocks) {
      html += '<div class="cfg-block" data-block="' + esc(block.name) + '">';
      html += '<div class="cfg-block-header">' + esc(block.name);
      if (block.isEntry) html += '<span class="tag tag-entry">entry</span>';
      html += '</div>';
      for (var stmt of block.statements) {
        html += '<div class="cfg-stmt"' + sourceRangeAttrs(stmt.range) + '><span class="idx"></span><span class="' + stmt.colorClass + '">' + esc(stmt.summary) + '</span> <span style="color:var(--text-dim); font-size:10px">[' + spanLabel(stmt.range) + ']</span></div>';
      }
      if (block.terminator) {
        html += '<div class="cfg-terminator"' + sourceRangeAttrs(block.terminator.range) + '>' + renderTerminator(block.terminator) + '</div>';
      }
      html += '</div>';
    }
    html += '</div>';
  }
  pane.innerHTML = html || '<div class="empty-state">No functions</div>';
  setupHoverHighlighting(pane);
  setupOptToolbar(pane, 'pane-cfg-pre', renderCfgPre);
  requestAnimationFrame(function() { drawAllCfgEdges(pane, funcs); });
}
function renderTerminator(t) {
  if (t.type === 'goto') return 'goto ' + esc(t.target);
  if (t.type === 'branch') return 'branch <span style="color:var(--text)">' + esc(t.condition) + '</span> \u2192 ' + esc(t.trueTarget) + ' / ' + esc(t.falseTarget);
  if (t.type === 'return') return 'return' + (t.value ? ' ' + esc(t.value) : '');
  return '';
}

// CFG (post-SSA)
function renderCfgPost() {
  var pane = $('#pane-cfg-post');
  var optAvail = !!data.cfgPostSsaOptimised;
  var mode = optAvail ? (structOptState['pane-cfg-post'] || 'off') : 'off';
  var toolbar = renderOptToolbar('pane-cfg-post', optAvail);
  if (mode === 'diff') {
    pane.innerHTML = toolbar + renderTextDiff(cfgToLines(data.cfgPostSsa), cfgToLines(data.cfgPostSsaOptimised));
    setupOptToolbar(pane, 'pane-cfg-post', renderCfgPost);
    return;
  }
  var funcs = mode === 'on' ? data.cfgPostSsaOptimised : data.cfgPostSsa;
  var html = toolbar;
  for (var func of funcs) {
    html += '<div class="cfg-function cfg-edges-container" data-func="' + esc(func.name) + '">';
    html += '<div class="cfg-func-header">' + esc(func.name) + ' <span style="color:var(--text-dim); font-size:11px">entry=' + esc(func.entry) + ' blocks=' + func.blockCount + '</span></div>';
    for (var block of func.blocks) {
      var cls = block.isUnreachable ? 'cfg-block unreachable' : 'cfg-block';
      html += '<div class="' + cls + '" data-block="' + esc(block.name) + '">';
      html += '<div class="cfg-block-header">' + esc(block.name);
      if (block.isEntry) html += '<span class="tag tag-entry">entry</span>';
      if (block.isUnreachable) html += '<span class="tag tag-unreachable">unreachable</span>';
      html += '</div>';
      for (var phi of block.phis) {
        var incoming = Object.entries(phi.incoming).map(function(e) { return e[0] + ':' + e[1]; }).join(', ');
        html += '<div class="cfg-phi">' + renderVarSpan(phi.name, phi.version, phi.type, null, 'def') + ' \u2190 ' + esc(incoming) + '</div>';
      }
      for (var stmt of block.statements) {
        html += '<div class="cfg-stmt"' + sourceRangeAttrs(stmt.range) + '><span class="' + stmt.colorClass + '">' + esc(stmt.summary) + '</span> <span style="color:var(--text-dim); font-size:10px">[' + spanLabel(stmt.range) + ']</span></div>';
        html += renderSSAInfo(stmt.uses, stmt.defs);
      }
      if (block.terminator) {
        html += '<div class="cfg-terminator"' + sourceRangeAttrs(block.terminator.range) + '>' + renderTerminator(block.terminator) + '</div>';
      }
      html += '</div>';
    }
    var a = func.analysis;
    html += '<div class="analysis-card">';
    html += '<h3>' + esc(func.name) + ' analysis</h3>';
    if (a.constantBranches.length) {
      html += '<div class="analysis-entry" style="color:var(--blue)">constant branches:</div>';
      for (var b of a.constantBranches) {
        html += '<div class="analysis-entry" style="margin-left:12px; color:var(--blue)">' + esc(b.block) + ': ' + esc(b.condition) + ' is always <span class="val">' + b.value + '</span> (take ' + esc(b.takenTarget) + ')</div>';
      }
    }
    if (a.deadStores.length) {
      html += '<div class="analysis-entry" style="color:var(--yellow)">dead stores:</div>';
      for (var d of a.deadStores) {
        html += '<div class="analysis-entry" style="margin-left:12px; color:var(--yellow)">' + esc(d.block) + ' stmt#' + d.stmtIndex + ': ' + esc(d.variable) + '#' + d.version + '</div>';
      }
    }
    if (a.unreachableBlocks.length) {
      html += '<div class="analysis-entry" style="color:var(--magenta)">unreachable: <span class="val">' + esc(a.unreachableBlocks.join(', ')) + '</span></div>';
    }
    if (Object.keys(a.inferredTypes).length) {
      html += '<div class="analysis-entry" style="color:var(--green)">inferred types:</div>';
      for (var _e2 of Object.entries(a.inferredTypes)) {
        html += '<div class="analysis-entry" style="margin-left:12px; color:var(--green)">' + esc(_e2[0]) + ': <span class="val">' + esc(_e2[1]) + '</span></div>';
      }
    }
    html += '</div></div>';
  }
  pane.innerHTML = html || '<div class="empty-state">No functions</div>';
  setupHoverHighlighting(pane);
  setupVarTooltips(pane);
  setupOptToolbar(pane, 'pane-cfg-post', renderCfgPost);
  requestAnimationFrame(function() { drawAllCfgEdges(pane, funcs); });
}

// Unit scope: who else can call this file's procedures (issue #977).
// Rendered at the top of the Interproc pane because it is the precondition
// for every interprocedural constant seed below it — the `seeding` line says
// whether the seeds were allowed, and which boundary declined them.
function renderUnitScope() {
  var scope = data.unitScope;
  if (!scope) return '';
  var boundaries = (scope.boundaries && scope.boundaries.length)
    ? esc(scope.boundaries.join(', '))
    : 'none';
  var html = '<div class="proc-card">';
  html += '<div class="proc-name">unit scope';
  html += '<span class="pure-badge ' + (scope.hasCrossFileEvidence ? 'pure-yes' : 'pure-no') + '">'
    + (scope.hasCrossFileEvidence ? 'cross-file view' : 'single file') + '</span>';
  html += '</div>';
  html += '<div class="proc-detail">registry boundaries: <span class="val">' + boundaries + '</span></div>';
  html += '<div class="proc-detail">seeding: <span class="val">' + esc(scope.seeding || '') + '</span></div>';
  for (var c of (scope.callees || [])) {
    var parts = [];
    for (var pos of (c.positions || [])) parts.push('arg ' + esc(pos.index) + ': ' + esc(pos.verdict));
    html += '<div class="proc-detail">' + esc(c.name) + ': <span class="val">'
      + (parts.length ? parts.join('; ') : '\u2014') + '</span></div>';
  }
  html += '</div>';
  return html;
}

// Interprocedural
function renderInterproc() {
  var pane = $('#pane-interproc');
  var scopeHtml = renderUnitScope();
  if (!data.interprocedural.length) {
    pane.innerHTML = scopeHtml + '<div class="empty-state">No procedures to analyse</div>';
    return;
  }
  var html = scopeHtml;
  for (var p of data.interprocedural) {
    // TclOO method summaries carry a different shape than proc summaries
    // (no arity/foldable/returnShape; instead methodKind + instance-var
    // writes).  Render them with their own layout so the proc branch can
    // assume its keys exist.
    if (p.kind === 'method') {
      html += '<div class="proc-card">';
      html += '<div class="proc-name">' + esc(p.name) + ' <span style="color:var(--text-dim); font-size:11px">' + esc(p.methodKind || 'method') + '</span>';
      html += '<span class="pure-badge ' + (p.pure ? 'pure-yes' : 'pure-no') + '">' + (p.pure ? 'pure' : 'impure') + '</span>';
      html += '</div>';
      html += '<div class="proc-detail">calls: <span class="val">' + (p.calls && p.calls.length ? esc(p.calls.join(', ')) : '—') + '</span></div>';
      if (p.writesInstanceVars && p.writesInstanceVars.length) html += '<div class="proc-detail">writes-instance-vars: <span class="val">' + esc(p.writesInstanceVars.join(', ')) + '</span></div>';
      var mflags = [];
      if (p.hasBarrier) mflags.push('barrier');
      if (p.hasUnknownCalls) mflags.push('unknown_calls');
      if (p.writesGlobal) mflags.push('writes_global');
      if (mflags.length) html += '<div class="proc-detail">flags: <span class="val">' + esc(mflags.join(', ')) + '</span></div>';
      html += '</div>';
      continue;
    }
    html += '<div class="proc-card">';
    html += '<div class="proc-name">' + esc(p.name) + ' <span style="color:var(--text-dim); font-size:11px">arity=' + esc(p.arity) + '</span>';
    html += '<span class="pure-badge ' + (p.pure ? 'pure-yes' : 'pure-no') + '">' + (p.pure ? 'pure' : 'impure') + '</span>';
    if (p.foldable) html += '<span class="pure-badge pure-yes">foldable</span>';
    html += '</div>';
    html += '<div class="proc-detail">return: <span class="val">' + esc(p.returnShape) + '</span></div>';
    html += '<div class="proc-detail">calls: <span class="val">' + (p.calls.length ? esc(p.calls.join(', ')) : '\u2014') + '</span></div>';
    // The caller-uniform-literal SCCP seed this proc was analysed under —
    // the fact that explains a folded condition on a parameter, and (by its
    // absence) an indirect call site the scan could not enumerate.
    if (p.paramConstants && p.paramConstants.length) html += '<div class="proc-detail">param-constants: <span class="val">' + esc(p.paramConstants.join(', ')) + '</span></div>';
    var flags = [];
    if (p.hasBarrier) flags.push('barrier');
    if (p.hasUnknownCalls) flags.push('unknown_calls');
    if (p.writesGlobal) flags.push('writes_global');
    if (flags.length) html += '<div class="proc-detail">flags: <span class="val">' + esc(flags.join(', ')) + '</span></div>';
    html += '</div>';
  }
  pane.innerHTML = html;
}

// Optimiser
function renderOpt() {
  var pane = $('#pane-opt');
  if (!data.optimisations.length) { pane.innerHTML = '<div class="empty-state">No optimiser rewrites</div>'; return; }
  var html = '<div class="section-header">Rewrites</div>';
  for (var o of data.optimisations) {
    html += '<div class="opt-item"' + sourceRangeAttrs(o.range) + '><span class="opt-code">' + esc(o.code) + '</span><span class="opt-msg">' + esc(o.message) + ' <span style="color:var(--text-dim); font-size:10px">[' + spanLabel(o.range) + ']</span></span><span class="opt-repl">\u2192 ' + esc(o.replacement) + '</span></div>';
  }
  if (data.optimisedSource) { html += '<div class="section-header">Source Diff</div>' + buildOptDiffView(); }
  pane.innerHTML = html;
  setupHoverHighlighting(pane);
  if (data.optimisedSource) { requestAnimationFrame(function() { drawOptBrackets(pane); }); setupOptDiffHover(pane); setupOptItemDiffScroll(pane); }
}

// Rust-native: the optimiser pass pipeline — each pass in execution order
// with the rewrites it produced. Absent when served by the Python backend.
function renderOptimiserPasses() {
  var pane = $('#pane-optimiser-passes');
  if (!pane) return;
  var passes = data.optimiserPasses;
  if (!passes || !passes.length) { pane.innerHTML = '<div class="empty-state">Optimiser pipeline unavailable</div>'; return; }
  var html = '';
  for (var p of passes) {
    html += '<div class="section-header">' + esc(p.label) + ' <span style="color:var(--text-dim); font-weight:normal">' + esc(p.id) + ' &middot; ' + p.count + ' rewrite' + (p.count === 1 ? '' : 's') + '</span></div>';
    if (!p.optimisations.length) {
      html += '<div class="analysis-entry" style="color:var(--text-dim); margin-left:8px">(no rewrites)</div>';
      continue;
    }
    for (var o of p.optimisations) {
      html += '<div class="opt-item"' + sourceRangeAttrs(o.range) + '><span class="opt-code">' + esc(o.code) + '</span><span class="opt-msg">' + esc(o.message) + ' <span style="color:var(--text-dim); font-size:10px">[' + spanLabel(o.range) + ']</span></span><span class="opt-repl">→ ' + esc(o.replacement) + '</span></div>';
    }
  }
  pane.innerHTML = html;
  setupHoverHighlighting(pane);
}

function computeDiffSegments(origLines, optLines) {
  var segments = [];
  if (origLines.length === optLines.length) {
    var i = 0;
    while (i < origLines.length) {
      if (origLines[i] === optLines[i]) {
        var start = i;
        while (i < origLines.length && origLines[i] === optLines[i]) i++;
        segments.push({type:'same',origStart:start,origEnd:i,optStart:start,optEnd:i});
      } else {
        var start = i;
        while (i < origLines.length && origLines[i] !== optLines[i]) i++;
        segments.push({type:'changed',origStart:start,origEnd:i,optStart:start,optEnd:i});
      }
    }
  } else {
    var lcs = computeLCS(origLines, optLines);
    var oi=0,ni=0,li=0;
    while (oi<origLines.length||ni<optLines.length) {
      if (li<lcs.length&&oi===lcs[li][0]&&ni===lcs[li][1]) {
        var oS=oi,nS=ni;
        while(li<lcs.length&&oi===lcs[li][0]&&ni===lcs[li][1]){oi++;ni++;li++;}
        segments.push({type:'same',origStart:oS,origEnd:oi,optStart:nS,optEnd:ni});
      } else {
        var oS=oi,nS=ni;
        var oE=li<lcs.length?lcs[li][0]:origLines.length;
        var nE=li<lcs.length?lcs[li][1]:optLines.length;
        if(oS<oE||nS<nE) segments.push({type:'changed',origStart:oS,origEnd:oE,optStart:nS,optEnd:nE});
        oi=oE;ni=nE;
      }
    }
  }
  return segments;
}

function computeLCS(a,b) {
  var m=a.length,n=b.length;
  if(m*n>250000)return[];
  var dp=Array.from({length:m+1},function(){return new Uint16Array(n+1)});
  for(var i=1;i<=m;i++)for(var j=1;j<=n;j++)dp[i][j]=a[i-1]===b[j-1]?dp[i-1][j-1]+1:Math.max(dp[i-1][j],dp[i][j-1]);
  var result=[];var i=m,j=n;
  while(i>0&&j>0){if(a[i-1]===b[j-1]){result.push([i-1,j-1]);i--;j--;}else if(dp[i-1][j]>dp[i][j-1])i--;else j--;}
  result.reverse();return result;
}

function drawOptBrackets(pane) {
  var container = pane.querySelector('.opt-diff-container');
  if (!container) return;
  var groupIds = [];
  container.querySelectorAll('[data-opt-group]').forEach(function(el) {
    var gid = el.dataset.optGroup;
    if (groupIds.indexOf(gid) < 0) groupIds.push(gid);
  });
  var edges = [];
  for (var idx = 0; idx < groupIds.length; idx++) {
    var gid = groupIds[idx];
    var origEls = container.querySelectorAll('.opt-original[data-opt-group="' + gid + '"]');
    var replEls = container.querySelectorAll('.opt-replacement[data-opt-group="' + gid + '"]');
    if (!origEls.length || !replEls.length) continue;
    edges.push({
      from: origEls[0],
      to: replEls[replEls.length - 1],
      fromId: gid,
      toId: gid,
      fromPos: idx,
      toPos: idx,
      kind: 'bracket',
      directed: false,
    });
  }
  drawOrthogonalEdges(container, edges, {
    svgClass: 'opt-diff-svg',
    edgeClass: 'opt-bracket',
    edgeKindClass: function() { return ''; },
    markerKinds: [],  // brackets have no arrowhead
    gutter: { laneWidth: 8, innerX: 14, entryX: 32, exitX: 32, cornerRadius: 4, minX: 6 },
    endpointSelector: '[data-opt-group]',
    endpointIdAttr: 'optGroup',
  });
  // Preserve the legacy data-opt-group attr so existing hover/click
  // hooks in opt-item/diff wiring still resolve groups.
  container.querySelectorAll('.opt-bracket').forEach(function(p) {
    if (!p.dataset.optGroup && p.dataset.edgeFrom) p.dataset.optGroup = p.dataset.edgeFrom;
  });
}

function clearOptHighlights(container) {
  container.querySelectorAll('.opt-input-highlight').forEach(function(el){el.classList.remove('opt-input-highlight')});
  container.querySelectorAll('.opt-output-highlight').forEach(function(el){el.classList.remove('opt-output-highlight')});
  container.querySelectorAll('.opt-bracket.highlighted').forEach(function(el){el.classList.remove('highlighted')});
}

// GVN
function renderGvn() {
  var pane=$('#pane-gvn');
  if(!data.gvn.length){pane.innerHTML='<div class="empty-state">No redundant computations detected</div>';return;}
  var html='';
  for(var w of data.gvn){
    var head='<div class="gvn-item"'+sourceRangeAttrs(w.range)+'><span class="gvn-code">'+esc(w.code)+'</span> '+esc(w.message||'redundant computation')+' <span class="gvn-expr">'+esc(w.expression)+'</span> <span class="gvn-first">[first: '+spanLabel(w.firstRange)+']</span> <span style="color:var(--text-dim); font-size:10px">['+spanLabel(w.range)+']</span></div>';
    var detail=detailRow('code',w.code)+detailRow('message',w.message||'redundant computation')+detailRow('expression',w.expression)+detailRow('first seen',spanLabel(w.firstRange))+rangeDetail(w.range);
    html+=xpand(head,detail);
  }
  pane.innerHTML=html;setupHoverHighlighting(pane);setupExpandable(pane);
}

// Taint
function renderTaint() {
  var pane=$('#pane-taint');
  var hasWarnings=data.taintWarnings.length>0;
  var hasTracking=data.taintTracking.length>0;
  if(!hasWarnings&&!hasTracking){pane.innerHTML='<div class="empty-state">No tainted data flows detected</div>';return;}
  var html='';
  if(hasWarnings){
    html+='<div class="section-header">Warnings</div>';
    for(var w of data.taintWarnings){
      // ``severity`` is set by the backend (tooling/explorer/annotations.taint_severity);
      // the renderer just maps it to a CSS class.  Falls back to "warning"
      // when the backend didn't classify (older payloads / unit tests).
      var severity = w.severity || 'warning';
      var head='<div class="taint-item taint-'+severity+'"'+sourceRangeAttrs(w.range)+'><span class="taint-code">'+esc(w.code)+'</span> '+esc(w.message)+' ';
      if(w.variable)head+='<span style="color:var(--orange)">'+esc(w.variable)+'</span> ';
      if(w.sinkCommand)head+='<span style="color:var(--red)">\u2192 '+esc(w.sinkCommand)+'</span> ';
      head+='<span style="color:var(--text-dim); font-size:10px">['+spanLabel(w.range)+']</span></div>';
      var detail=detailRow('code',w.code)+detailRow('severity',severity)+detailRow('message',w.message)+
        (w.variable?detailRow('variable',w.variable):'')+(w.sinkCommand?detailRow('sink command',w.sinkCommand):'')+rangeDetail(w.range);
      html+=xpand(head,detail);
    }
  }
  if(hasTracking){
    html+='<div class="section-header">Taint Tracking</div>';
    for(var func of data.taintTracking){
      html+='<div class="proc-card">';
      html+='<div class="proc-name">'+esc(func.name)+'</div>';
      for(var e of func.entries){
        var thead='<div class="taint-tracking-var">'+esc(e.variable)+'#'+e.version+': <span class="taint-val">'+esc(e.taint)+'</span></div>';
        html+=xpand(thead,detailRow('variable',e.variable)+detailRow('version',e.version)+detailRow('taint',e.taint));
      }
      html+='</div>';
    }
  }
  pane.innerHTML=html;setupHoverHighlighting(pane);setupExpandable(pane);
}

// Types
function renderTypes() {
  var pane=$('#pane-types');
  if(!data.types.length){pane.innerHTML='<div class="empty-state">No type information inferred</div>';return;}
  var html='';
  for(var func of data.types){
    html+='<div class="proc-card">';
    html+='<div class="proc-name">'+esc(func.name)+'</div>';
    for(var e of func.entries){
      var label=e.variable.startsWith('(')?e.variable:e.variable+'#'+e.version;
      var head='<div class="type-entry"><span class="type-var">'+esc(label)+'</span><span class="type-val type-'+e.kind+'">'+esc(e.type)+'</span></div>';
      var detail=detailRow('variable',e.variable)+detailRow('version',e.version)+detailRow('kind',e.kind)+detailRow('type',e.type);
      html+=xpand(head,detail);
    }
    html+='</div>';
  }
  pane.innerHTML=html;
  setupExpandable(pane);
}

// Rendered Properties
function renderRendered() {
  var pane=$('#pane-rendered');
  var rp=data.renderedProperties||[];
  if(!rp.length){pane.innerHTML='<div class="empty-state">No rendered value properties</div>';return;}
  var html='';
  for(var func of rp){
    html+='<div class="proc-card">';
    html+='<div class="proc-name">'+esc(func.name)+'</div>';
    for(var e of func.entries){
      var may=e.may.length?'may: '+e.may.join(', '):'';
      var must=e.must.length?'must: '+e.must.join(', '):'';
      var parts=[may,must].filter(function(x){return x;}).join(' | ');
      var head='<div class="type-entry"><span class="type-var">'+esc(e.variable)+'#'+e.version+'</span><span class="type-val" style="color:var(--cyan)">'+esc(parts)+'</span></div>';
      var detail=detailRow('variable',e.variable)+detailRow('version',e.version)+detailRow('may',e.may.join(', ')||'—')+detailRow('must',e.must.join(', ')||'—');
      html+=xpand(head,detail);
    }
    html+='</div>';
  }
  pane.innerHTML=html;
  setupExpandable(pane);
}

// Generic expand-on-click / Space|Enter for explorer items.
//
// Any element with class ``xpand`` toggles ``expanded`` when clicked or
// (when focused) on Space/Enter, revealing its direct ``.xpand-detail`` and
// ``.xpand-children``.  Markup gives expandable rows ``tabindex="0"`` so
// keyboard users can focus and expand them.  Clicks inside an already-open
// ``.xpand-detail`` are ignored so text stays selectable, and ``closest``
// resolves to the innermost row so nested trees toggle independently.
function setupExpandable(container) {
  if (container._xpandWired) return;
  container._xpandWired = true;
  container.addEventListener('click', function(e) {
    if (e.target.closest('a, button, input, textarea, .xpand-detail')) return;
    var row = e.target.closest('.xpand');
    if (row && container.contains(row)) row.classList.toggle('expanded');
  });
  container.addEventListener('keydown', function(e) {
    if (e.key !== ' ' && e.key !== 'Enter') return;
    var row = e.target.closest && e.target.closest('.xpand');
    if (row && row === document.activeElement) {
      e.preventDefault();
      row.classList.toggle('expanded');
    }
  });
}

function detailRow(k, v) {
  return '<div class="xpand-detail-row"><span class="xpand-detail-k">' + esc(k) +
         '</span><span class="xpand-detail-v">' + esc(String(v)) + '</span></div>';
}

// Non-destructively wrap an existing item's markup so it expands to a detail
// block on click / Space.  ``headHtml`` is the item exactly as it rendered
// before (keeping its class, data-start hover and other hooks intact); the
// detail is a sibling that only shows when the wrapper is expanded.  Lets us
// retrofit expand-to-detail onto a tab without restructuring its items.
function xpand(headHtml, detailHtml) {
  return '<div class="xpand xpand-wrap" tabindex="0">' + headHtml +
         '<div class="xpand-detail">' + detailHtml + '</div></div>';
}
function rangeDetail(range) {
  if (!range) return '';
  return detailRow('range', spanLabel(range) + '  (' + range.startOffset + '…' + range.endOffset + ')');
}

// Optimisation lens (off / on / diff) for the structured IR & CFG tabs.
// ``off`` renders the original program, ``on`` the optimised program, and
// ``diff`` a rendered-line diff of the two.  ASM/WASM have their own toggle
// (renderDisassembly); this mirrors it for the structured views.
var structOptState = { 'pane-ir': 'off', 'pane-cfg-pre': 'off', 'pane-cfg-post': 'off' };

function renderOptToolbar(paneId, available) {
  var mode = available ? (structOptState[paneId] || 'off') : 'off';
  function btn(val, label) {
    var dis = (val !== 'off' && !available) ? ' disabled' : '';
    var on = mode === val ? ' active' : '';
    return '<button class="optlens-btn' + on + '" data-opt-lens="' + val + '"' + dis + '>' + label + '</button>';
  }
  var hint = available ? '' : '<span class="disasm-toolbar-hint">(source unchanged by optimiser)</span>';
  return '<div class="disasm-toolbar optlens-toolbar">'
       + '<span class="optlens-label">Optimiser</span>'
       + btn('off', 'off') + btn('on', 'on') + btn('diff', 'diff') + hint
       + '</div>';
}
function setupOptToolbar(pane, paneId, rerender) {
  pane.querySelectorAll('[data-opt-lens]').forEach(function(b) {
    b.addEventListener('click', function() {
      if (b.disabled) return;
      structOptState[paneId] = b.dataset.optLens;
      rerender();
    });
  });
}

// Flatten the structured IR / CFG into plain text lines for a line diff.
function irToLines(ir) {
  var lines = [];
  function walk(nodes, depth) {
    for (var n of (nodes || [])) {
      lines.push('  '.repeat(depth) + n.summary);
      if (n.children) for (var c of n.children) {
        lines.push('  '.repeat(depth + 1) + c.label + ':');
        walk(c.body, depth + 2);
      }
    }
  }
  lines.push('top-level');
  walk(ir.topLevel, 1);
  if (ir.procedures) for (var name of Object.keys(ir.procedures)) {
    lines.push('proc ' + name);
    walk(ir.procedures[name].body, 1);
  }
  return lines;
}
function cfgToLines(funcs) {
  var lines = [];
  for (var f of (funcs || [])) {
    lines.push('function ' + f.name + ' entry=' + f.entry + ' blocks=' + f.blockCount);
    for (var b of f.blocks) {
      lines.push('block ' + b.name + (b.isEntry ? ' [entry]' : '') + (b.isUnreachable ? ' [unreachable]' : ''));
      if (b.phis) for (var p of b.phis) lines.push('  phi ' + p.name + '#' + p.version);
      for (var s of b.statements) lines.push('  ' + s.summary);
      if (b.terminator) {
        var t = b.terminator;
        lines.push('  term ' + t.type + (t.target ? ' ' + t.target : '') +
                   (t.trueTarget ? ' ' + t.trueTarget + '/' + t.falseTarget : '') +
                   (t.value ? ' ' + t.value : ''));
      }
    }
  }
  return lines;
}
function renderTextDiff(origLines, optLines) {
  var segs = computeDiffSegments(origLines, optLines);
  if (!segs.some(function(s) { return s.type !== 'same'; })) {
    return '<div class="empty-state">No changes under the optimiser</div>';
  }
  var html = '<div class="optlens-legend"><span class="optlens-del-k">− original</span> <span class="optlens-add-k">+ optimised</span></div>';
  html += '<pre class="optlens-diff">';
  for (var seg of segs) {
    if (seg.type === 'same') {
      var n = seg.optEnd - seg.optStart, show = Math.min(n, 1);
      for (var i = 0; i < show; i++) html += '<div class="optlens-line"><span class="optlens-sig"> </span>' + esc(optLines[seg.optStart + i]) + '</div>';
      if (n > show) html += '<div class="optlens-elide">… ' + (n - show) + ' unchanged line' + ((n - show) === 1 ? '' : 's') + '</div>';
    } else {
      for (var i = seg.origStart; i < seg.origEnd; i++) html += '<div class="optlens-line optlens-del"><span class="optlens-sig">−</span>' + esc(origLines[i]) + '</div>';
      for (var j = seg.optStart; j < seg.optEnd; j++) html += '<div class="optlens-line optlens-add"><span class="optlens-sig">+</span>' + esc(optLines[j]) + '</div>';
    }
  }
  html += '</pre>';
  return html;
}

// Canonical red-green CST: DOCUMENT -> COMMAND -> WORD -> fragment, the
// structural tree the whole pipeline rides on (counterpart to the green tree).
function renderCst() {
  var pane = $('#pane-cst');
  var root = data.cst;
  if (!root) { pane.innerHTML = '<div class="empty-state">No CST</div>'; return; }
  var html = '<div class="section-header">cst <span style="color:var(--text-dim);font-weight:normal">' +
             esc(root.kind.toLowerCase()) + ' &middot; ' + Math.max(0, root.endOffset - root.startOffset) + ' bytes</span></div>';
  html += '<div class="gt-tree">' + (cstChildren(root.children) || '<div class="gt-empty">(no commands)</div>') + '</div>';
  pane.innerHTML = html;
  setupHoverHighlighting(pane);
  setupExpandable(pane);
}
function cstChildren(children) {
  if (!children || !children.length) return '';
  var html = '';
  for (var n of children) {
    html += (n.children !== undefined) ? cstNode(n) : cstLeaf(n);
  }
  return html;
}
function cstNode(n) {
  var kind = n.kind.toLowerCase();
  var label = (n.label || '').replace(/\n/g, '⏎');
  var preview = label.length > 48 ? label.slice(0, 48) + '…' : label;
  var tags = (n.tags && n.tags.length) ? '{' + n.tags.join(',') + '}' : '';
  var html = '<div class="gt-node xpand" tabindex="0" data-start="' + n.startOffset + '" data-end="' + n.endOffset + '">';
  html += '<div class="xpand-head gt-head">';
  html += '<span class="gt-toggle">▸</span>';
  html += '<span class="gt-type">' + esc(kind) + '</span>';
  html += '<span class="gt-text">' + esc(preview) + '</span>';
  if (tags) html += '<span class="gt-tags">' + esc(tags) + '</span>';
  html += '<span class="gt-range">[' + n.startOffset + ':' + n.endOffset + ']</span>';
  html += '</div>';
  html += '<div class="xpand-detail">';
  html += detailRow('kind', n.kind);
  html += detailRow('byte range', n.startOffset + '…' + n.endOffset + ' (' + Math.max(0, n.endOffset - n.startOffset) + ' bytes)');
  if (n.tags && n.tags.length) html += detailRow('shape', n.tags.join(', '));
  if (n.label) {
    html += '<div class="xpand-detail-row"><span class="xpand-detail-k">text</span></div>';
    html += '<pre class="gt-text-full">' + esc(n.label) + '</pre>';
  }
  html += '</div>';
  html += '<div class="xpand-children">' + cstChildren(n.children) + '</div>';
  html += '</div>';
  return html;
}
function cstLeaf(t) {
  var opaque = t.kind === 'STR' || t.kind === 'CMD';
  var hasChild = !!t.child;
  var recovered = hasChild && t.terminated === false;
  var cls = recovered ? 'gt-error' : (opaque ? 'gt-opaque' : '');
  var displayText = (t.isMarker ? t.raw : t.text) || '';
  var oneLine = displayText.replace(/\n/g, '⏎');
  var preview = oneLine.length > 48 ? oneLine.slice(0, 48) + '…' : oneLine;
  var html = '<div class="gt-node xpand ' + cls + '" tabindex="0" data-start="' + t.startOffset + '" data-end="' + t.endOffset + '">';
  html += '<div class="xpand-head gt-head">';
  html += '<span class="gt-toggle">' + (hasChild ? '▸' : '·') + '</span>';
  html += '<span class="gt-type">' + esc(t.kind) + '</span>';
  html += '<span class="gt-text">' + esc(preview) + '</span>';
  html += '<span class="gt-range">[' + t.startOffset + ':' + t.endOffset + ']</span>';
  html += '</div>';
  html += '<div class="xpand-detail">';
  html += detailRow('token type', t.kind + (t.isMarker ? ' (expansion marker)' : ''));
  html += detailRow('byte range', t.startOffset + '…' + t.endOffset + ' (' + Math.max(0, t.endOffset - t.startOffset) + ' bytes)');
  if (t.inQuote) html += detailRow('in quote', 'yes');
  if (t.raw !== t.text) {
    html += '<div class="xpand-detail-row"><span class="xpand-detail-k">raw</span></div>';
    html += '<pre class="gt-text-full">' + esc(t.raw) + '</pre>';
  }
  if (hasChild) html += detailRow('region', t.terminated ? 'terminated' : 'recovered (unterminated)');
  html += '<div class="xpand-detail-row"><span class="xpand-detail-k">text</span></div>';
  html += '<pre class="gt-text-full">' + esc(t.text || '') + '</pre>';
  html += '</div>';
  if (hasChild) {
    var c = t.child;
    html += '<div class="xpand-children">';
    html += '<div class="gt-region">body &middot; ' + Math.max(0, c.endOffset - c.startOffset) + ' bytes' +
            (t.terminated ? '' : ' &middot; <span style="color:var(--red)">RECOVERED</span>') + '</div>';
    html += cstChildren(c.children);
    html += '</div>';
  }
  html += '</div>';
  return html;
}

// SegmentedCommand list: the public segmenter contract every command consumer
// reads, derived from the CST (byte-identical to the analyser's view).
function renderSegments() {
  var pane = $('#pane-segments');
  var segs = data.segments;
  if (!segs || !segs.length) { pane.innerHTML = '<div class="empty-state">No commands</div>'; return; }
  var html = '<div class="section-header">segments <span style="color:var(--text-dim);font-weight:normal">' +
             segs.length + ' command' + (segs.length === 1 ? '' : 's') + '</span></div>';
  html += '<div class="gt-tree">';
  for (var seg of segs) {
    var slice = (seg.slice || '').replace(/\n/g, '⏎');
    var preview = slice.length > 48 ? slice.slice(0, 48) + '…' : slice;
    html += '<div class="gt-node xpand" tabindex="0" data-start="' + seg.startOffset + '" data-end="' + seg.endOffset + '">';
    html += '<div class="xpand-head gt-head">';
    html += '<span class="gt-toggle">▸</span>';
    html += '<span class="gt-type">' + esc(seg.name || '<anon>') + '</span>';
    html += '<span class="gt-text">' + esc(preview) + '</span>';
    html += '<span class="gt-range">[' + seg.startOffset + ':' + seg.endOffset + ']</span>';
    html += '</div>';
    html += '<div class="xpand-detail">';
    html += detailRow('byte range', seg.startOffset + '…' + seg.endOffset + ' (' + Math.max(0, seg.endOffset - seg.startOffset) + ' bytes)');
    if (seg.precedingComment) html += detailRow('comment', seg.precedingComment.replace(/\n/g, ' '));
    if (seg.subcommand) html += detailRow('subcommand', seg.subcommand);
    html += detailRow('words', String(seg.words.length));
    html += '</div>';
    html += '<div class="xpand-children">';
    for (var w of seg.words) {
      var flags = [];
      if (w.single) flags.push('single');
      if (w.braced) flags.push('braced');
      if (w.quoted) flags.push('quoted');
      if (w.expand) flags.push('{*}');
      var wt = (w.text || '').replace(/\n/g, '⏎');
      var wp = wt.length > 48 ? wt.slice(0, 48) + '…' : wt;
      html += '<div class="gt-node" data-start="' + w.startOffset + '" data-end="' + w.endOffset + '">';
      html += '<div class="gt-head">';
      html += '<span class="gt-toggle">·</span>';
      html += '<span class="gt-text">' + esc(wp || "''") + '</span>';
      if (flags.length) html += '<span class="gt-tags">{' + flags.join(',') + '}</span>';
      html += '<span class="gt-range">[' + w.startOffset + ':' + w.endOffset + ']</span>';
      html += '</div></div>';
    }
    html += '</div>';
    html += '</div>';
  }
  html += '</div>';
  pane.innerHTML = html;
  setupHoverHighlighting(pane);
  setupExpandable(pane);
}

// Rust-native: the lexer structural pre-scan (tcl-lexer::structural_index) —
// command boundaries, bracket/brace balance, and the inert spans where
// [ ] { } are literal. Absent when served by the Python backend.
function renderStructuralIndex() {
  var pane = $('#pane-structural-index');
  if (!pane) return;
  var si = data.structuralIndex;
  if (!si) { pane.innerHTML = '<div class="empty-state">Structural index unavailable</div>'; return; }
  var html = '';
  var col = si.scriptComplete ? 'var(--green)' : 'var(--yellow)';
  html += '<div class="analysis-entry">script complete: <span class="val" style="color:' + col + '">' + si.scriptComplete + '</span></div>';

  html += '<div class="section-header">command boundaries <span style="color:var(--text-dim);font-weight:normal">' + si.commandBoundaries.length + '</span></div>';
  for (var b of si.commandBoundaries) {
    html += '<div class="analysis-entry" style="margin-left:8px"><span class="val">offset ' + b.offset + '</span> <span style="color:var(--text-dim)">(' + (b.line + 1) + ':' + (b.col + 1) + ')</span></div>';
  }

  for (var g of [['brackets', '[ ]'], ['braces', '{ }']]) {
    var grp = si[g[0]];
    html += '<div class="section-header">' + g[1] + ' <span style="color:var(--text-dim);font-weight:normal">' + grp.unterminated + ' unterminated &middot; ' + grp.structuralEvents + ' structural &middot; ' + grp.inertSpans.length + ' inert</span></div>';
    for (var sp of grp.inertSpans) {
      var t = (sp.text || '').replace(/\n/g, '⏎');
      var tp = t.length > 48 ? t.slice(0, 48) + '…' : t;
      var scol = sp.terminated ? 'var(--text-dim)' : 'var(--red)';
      html += '<div class="analysis-entry" style="margin-left:8px; color:' + scol + '" data-start="' + sp.start + '" data-end="' + sp.end + '">inert [' + sp.start + ':' + sp.end + '] ' + (sp.terminated ? '' : '(unterminated) ') + '<span style="color:var(--text)">' + esc(tp) + '</span></div>';
    }
  }
  pane.innerHTML = html;
  setupHoverHighlighting(pane);
}

// Rust-native: the LineIndex span model (tcl-lexer::SourceMap) — the
// line-start table that powers O(1) offset↔line:col resolution, the
// reference for debugging range bugs. Absent on the Python backend.
function renderSourceMap() {
  var pane = $('#pane-source-map');
  if (!pane) return;
  var sm = data.sourceMap;
  if (!sm) { pane.innerHTML = '<div class="empty-state">Source map unavailable</div>'; return; }
  var html = '<div class="section-header">source map <span style="color:var(--text-dim);font-weight:normal">' + sm.byteLength + ' bytes &middot; ' + sm.lineCount + ' line' + (sm.lineCount === 1 ? '' : 's') + '</span></div>';
  html += '<div class="source-listing">';
  for (var ln of sm.lines) {
    html += '<div class="source-line" data-start="' + ln.start + '" data-end="' + ln.end + '">';
    html += '<span class="gutter">' + (ln.line + 1) + '</span>';
    html += '<span class="code-text">' + esc(ln.text) + '</span>';
    html += '<span class="gt-range" style="margin-left:auto; color:var(--text-dim)">[' + ln.start + ':' + ln.end + ']</span>';
    html += '</div>';
  }
  html += '</div>';
  pane.innerHTML = html;
  setupHoverHighlighting(pane);
}

function fmtBound(v, pos) { return v === null || v === undefined ? (pos ? '+∞' : '−∞') : String(v); }

// Natural-loop forest
function renderLoops() {
  var pane = $('#pane-loops');
  var fns = data.loops || [];
  if (!fns.reduce(function(n, f) { return n + f.loops.length; }, 0)) {
    pane.innerHTML = '<div class="empty-state">No natural loops</div>'; return;
  }
  var html = '';
  for (var f of fns) {
    if (!f.loops.length) continue;
    html += '<div class="proc-card"><div class="proc-name">' + esc(f.name) + '</div>';
    for (var lp of f.loops) {
      html += '<div class="xpand loop-item" tabindex="0"><div class="xpand-head"><span class="gt-toggle">▸</span> header <span class="val">' + esc(lp.header) + '</span> &middot; depth <span class="val">' + lp.depth + '</span> &middot; <span class="val">' + lp.blockCount + '</span> block(s)</div>';
      html += '<div class="xpand-detail">';
      html += detailRow('header block', lp.header);
      html += detailRow('nesting depth', lp.depth);
      html += detailRow('blocks (' + lp.blockCount + ')', lp.blocks.join(', '));
      html += detailRow('latches', lp.latches.join(', ') || '—');
      html += '</div></div>';
    }
    html += '</div>';
  }
  pane.innerHTML = html;
  setupExpandable(pane);
}

// Integer-interval abstract domain
function renderIntervals() {
  var pane = $('#pane-intervals');
  var fns = data.intervals || [];
  if (!fns.reduce(function(n, f) { return n + f.entries.length; }, 0)) {
    pane.innerHTML = '<div class="empty-state">No bounded ranges</div>'; return;
  }
  var html = '';
  for (var f of fns) {
    if (!f.entries.length) continue;
    html += '<div class="proc-card"><div class="proc-name">' + esc(f.name) + '</div>';
    for (var e of f.entries) {
      var lo = fmtBound(e.lo, false), hi = fmtBound(e.hi, true);
      html += '<div class="xpand type-entry" tabindex="0"><div class="xpand-head"><span class="type-var">' + esc(e.variable) + '#' + e.version + '</span><span class="type-val">[' + lo + ', ' + hi + ']</span></div>';
      html += '<div class="xpand-detail">' + detailRow('lower bound', lo) + detailRow('upper bound', hi) + detailRow('ssa value', e.variable + '#' + e.version) + '</div></div>';
    }
    html += '</div>';
  }
  pane.innerHTML = html;
  setupExpandable(pane);
}

// Interval-driven bounds / divide-by-zero findings
function renderBounds() {
  var pane = $('#pane-bounds');
  var fns = data.bounds || [];
  if (!fns.reduce(function(n, f) { return n + f.findings.length + f.divzero.length; }, 0)) {
    pane.innerHTML = '<div class="empty-state">No provable out-of-range / divide-by-zero</div>'; return;
  }
  var html = '';
  for (var f of fns) {
    if (!f.findings.length && !f.divzero.length) continue;
    html += '<div class="proc-card"><div class="proc-name">' + esc(f.name) + '</div>';
    for (var b of f.findings) {
      var lo = fmtBound(b.lo, false), hi = fmtBound(b.hi, true);
      html += '<div class="xpand bounds-item" tabindex="0"><div class="xpand-head"><span class="shimmer-code">' + esc(b.code) + '</span> ' + esc(b.command) + ' $' + esc(b.indexVar) + ' in [' + lo + ', ' + hi + '] vs length ' + b.length + '</div>';
      html += '<div class="xpand-detail">' + detailRow('code', b.code) + detailRow('command', b.command) + detailRow('index var', '$' + b.indexVar) + detailRow('index range', '[' + lo + ', ' + hi + ']') + detailRow('container length', b.length) + detailRow('reason', b.reason) + '</div></div>';
    }
    for (var dz of f.divzero) {
      html += '<div class="xpand bounds-item" tabindex="0"><div class="xpand-head"><span class="shimmer-code">' + esc(dz.code) + '</span> \'' + esc(dz.op) + '\' divisor is provably 0 (divide by zero)</div>';
      html += '<div class="xpand-detail">' + detailRow('code', dz.code) + detailRow('operator', dz.op) + detailRow('reason', 'divisor interval is exactly [0, 0]') + '</div></div>';
    }
    html += '</div>';
  }
  pane.innerHTML = html;
  setupExpandable(pane);
}

// iRules event firing order
function renderEventOrder() {
  var pane = $('#pane-event-order');
  var events = (data.eventOrder || []).slice();
  if (!events.length) { pane.innerHTML = '<div class="empty-state">No event-order data</div>'; return; }
  events.sort(function(a, b) {
    return (b.base_priority + b.priority_offset) - (a.base_priority + a.priority_offset) || (a.event < b.event ? -1 : 1);
  });
  var html = '<div class="proc-card"><div class="proc-name">Event firing order (high → low priority)</div>';
  for (var e of events) {
    var eff = e.base_priority + e.priority_offset;
    var mult = e.multiplicity && e.multiplicity !== 'once' ? ' [' + esc(e.multiplicity) + ']' : '';
    html += '<div class="xpand type-entry" tabindex="0"' + sourceRangeAttrs(e.range) + '><div class="xpand-head"><span class="type-var">' + esc(e.event) + '</span><span class="type-val">eff ' + eff + mult + '</span></div>';
    html += '<div class="xpand-detail">' + detailRow('event', e.event) + detailRow('effective priority', eff) + detailRow('base priority', e.base_priority) + detailRow('priority offset', e.priority_offset) + detailRow('multiplicity', e.multiplicity || 'once') + '</div></div>';
  }
  html += '</div>';
  pane.innerHTML = html;
  setupHoverHighlighting(pane);
  setupExpandable(pane);
}

// Data Flow
function renderDataFlow() {
  var pane=$('#pane-dataflow');
  if(!data.dataflow||!data.dataflow.functions.length){pane.innerHTML='<div class="empty-state">No data-flow information</div>';return;}
  var html='';
  var s=data.dataflow.summary;
  html+='<div class="section-header">Summary: '+s.totalDefs+' defs, '+s.totalUses+' uses, '+s.totalAliases+' aliases</div>';
  for(var func of data.dataflow.functions){
    html+='<div class="proc-card">';
    html+='<div class="proc-name">'+esc(func.name)+' <span style="color:var(--text-dim); font-size:11px">'+func.summary.totalDefs+' defs, '+func.summary.totalUses+' uses</span>';
    if(func.summary.deadDefs>0)html+='<span class="pure-badge pure-no">'+func.summary.deadDefs+' dead</span>';
    if(func.summary.aliasedVars>0)html+='<span class="pure-badge" style="background:var(--orange);color:#000">'+func.summary.aliasedVars+' aliased</span>';
    html+='</div>';
    // Aliases
    if(func.aliases.length){
      html+='<div class="analysis-entry" style="color:var(--orange)">aliases:</div>';
      for(var a of func.aliases){
        var lk=a.localKind?a.localKind.toLowerCase():'';var tk=a.targetKind?a.targetKind.toLowerCase():'';
        html+='<div class="analysis-entry" style="margin-left:12px; color:var(--orange)">'+(lk?lk+'(':'')+ esc(a.localName)+(lk?')':'')+' &harr; '+(tk?tk+'(':'')+ esc(a.targetName)+(tk?')':'')+' <span style="color:var(--text-dim)">('+esc(a.reason)+')</span></div>';
      }
    }
    // Nodes (def-use chains)
    html+='<div class="analysis-entry" style="color:var(--cyan)">def-use chains:</div>';
    for(var n of func.nodes){
      var cls=n.isDead?'color:var(--yellow)':'color:var(--green)';
      var head='<div class="analysis-entry" style="margin-left:12px; '+cls+'">';
      head+=esc(n.name)+'#'+n.version;
      head+=' <span style="color:var(--text-dim)">['+esc(n.defKind)+' in '+esc(n.block)+']</span>';
      if(n.lattice)head+=' = <span class="val">'+esc(n.lattice)+'</span>';
      if(n.typeInfo&&n.typeInfo!=='UNKNOWN')head+=' : <span class="val">'+esc(n.typeInfo)+'</span>';
      head+=' &rarr; '+n.useCount+' use'+(n.useCount!==1?'s':'');
      if(n.isDead)head+=' <span style="color:var(--red)">(DEAD)</span>';
      head+='</div>';
      var detail=detailRow('ssa value',n.name+'#'+n.version)+detailRow('def kind',n.defKind)+detailRow('block',n.block)+
        (n.lattice?detailRow('lattice',n.lattice):'')+(n.typeInfo?detailRow('type',n.typeInfo):'')+
        detailRow('uses',n.useCount)+detailRow('dead',n.isDead?'yes':'no');
      html+=xpand(head,detail);
    }
    // Edges summary
    if(func.edges.length){
      var phiEdges=func.edges.filter(function(e){return e.edgeKind==='phi'}).length;
      var directEdges=func.edges.filter(function(e){return e.edgeKind==='direct'}).length;
      html+='<div class="analysis-entry" style="color:var(--blue)">edges: '+directEdges+' direct, '+phiEdges+' phi</div>';
    }
    html+='</div>';
  }
  pane.innerHTML=html;
  setupExpandable(pane);
}

// Source callouts
//
// The backend pre-groups annotations by source line (data.annotationsByLine,
// see tooling/cli/serialise._group_annotations_by_line); we just iterate
// the source lines and look up which annotation indices apply.  No
// client-side line indexing, no source parsing.
function renderCallouts() {
  var pane=$('#pane-callouts');
  if(!data.annotations.length){pane.innerHTML='<div class="empty-state">No annotations</div>';return;}
  var source=getSource();var lines=source.split('\n');
  // Per-line lineStart array — recomputed locally because the marker
  // arithmetic needs offset → column, which is presentation, not data.
  var lineStarts=[0];
  for(var i=0;i<source.length;i++){if(source[i]==='\n')lineStarts.push(i+1);}
  var byLine = data.annotationsByLine || {};
  var html='';var gutterWidth=String(lines.length).length;
  for(var i=0;i<lines.length;i++){
    html+='<div class="callout-line"><span class="gutter">'+String(i+1).padStart(gutterWidth)+'</span><span class="code-text">'+esc(lines[i])+'</span></div>';
    var indices = byLine[String(i)];
    if (!indices) continue;
    for (var idx of indices) {
      var ann = data.annotations[idx];
      var lineStart=lineStarts[i];var startCol=Math.max(0,ann.range.startOffset-lineStart);var endCol=Math.max(startCol,ann.range.endOffset-lineStart);
      var marker=' '.repeat(startCol)+'^'+'─'.repeat(Math.max(0,endCol-startCol));
      var arrow=' '.repeat(startCol)+'╰─▶ '+ann.label;
      // Both ``kind`` (source) and ``severity`` (classification) drive
      // CSS — kind colours the per-source visual identity, severity
      // gives danger/warn/info a consistent treatment across panes.
      var sevClass = ann.severity ? (' severity-' + ann.severity) : '';
      html+='<div class="callout-annotation kind-'+ann.kind+sevClass+'"'+sourceRangeAttrs(ann.range)+'>'+esc(marker)+'\n'+esc(arrow)+'</div>';
    }
  }
  pane.innerHTML=html;setupHoverHighlighting(pane);
}

// Assembly / WASM helpers
function instrCount(arr) { return arr ? arr.reduce(function(n, f) { return n + (f.instrCount || 0); }, 0) : 0; }
function funcListHtml(funcs) {
  var html = '';
  for (var func of funcs) {
    html += '<div class="cfg-function">';
    html += '<div class="cfg-func-header">' + esc(func.name) + ' <span style="color:var(--text-dim); font-size:11px">' + func.instrCount + ' instructions</span></div>';
    html += '<pre class="asm-listing">' + esc(func.text) + '</pre>';
    html += '</div>';
  }
  return html;
}

// Per-pane opt toggle state: true = optimised, false = original.
var wasmOptState = { 'pane-asm': false, 'pane-wasm': false };

// Tcl ASM and WASM share the same structured rendering pipeline.  The
// only differences are the data source (``data.asm`` vs ``data.wasm``)
// and a few WASM-specific node shapes (the ``(module)`` header,
// ``callTarget``, ``branchTarget``).  The renderer below handles both.
function renderAsm() { renderDisassembly('pane-asm', 'asm'); }
function renderWasm() { renderDisassembly('pane-wasm', 'wasm'); }

function renderDisassembly(paneId, kind) {
  var pane = $('#' + paneId);
  var origFuncs = kind === 'asm' ? data.asm : data.wasm;
  var optFuncs = kind === 'asm' ? data.asmOptimised : data.wasmOptimised;
  var emptyMsg = kind === 'asm' ? 'No assembly' : 'No WASM output';
  if (!origFuncs || !origFuncs.length) {
    pane.innerHTML = '<div class="empty-state">' + emptyMsg + '</div>';
    return;
  }
  var optAvailable = !!(optFuncs && optFuncs.length);
  // Preserve user's prior opt-toggle choice across re-renders.
  var optOn = optAvailable && wasmOptState[paneId] === true;
  var funcs = optOn ? optFuncs : origFuncs;
  var hasStructured = funcs.some(function(f) { return Array.isArray(f.instructions); });
  if (!hasStructured) {
    // Legacy text-only payload — plain <pre>.
    pane.innerHTML = funcListHtml(funcs);
    return;
  }
  var toolbar = renderDisasmToolbar(paneId, kind, optAvailable, optOn);
  var body = funcs.map(function(entry) { return renderDisasmEntry(entry, kind); }).join('');
  pane.innerHTML = toolbar + '<div class="disasm-body">' + body + '</div>';
  setupDisasmInteractions(pane, funcs, kind);
  setupDisasmToolbar(pane, paneId, kind, optAvailable);
  requestAnimationFrame(function() {
    pane.querySelectorAll('.wasm-edges-container').forEach(function(c) { drawWasmEdges(c); });
  });
}

function renderDisasmToolbar(paneId, kind, optAvailable, optOn) {
  var optLabel = kind === 'asm' ? 'Tcl ASM optimisations' : 'WASM optimisations';
  var disabledAttr = optAvailable ? '' : ' disabled';
  var hint = optAvailable ? '' : ' <span class="disasm-toolbar-hint">(source unchanged by optimiser)</span>';
  return '<div class="disasm-toolbar">'
       + '<label class="disasm-toolbar-opt"><input type="checkbox" data-disasm-opt-toggle' + (optOn ? ' checked' : '') + disabledAttr + '> ' + optLabel + '</label>'
       + '<button class="disasm-toolbar-diff" data-disasm-diff' + disabledAttr + '>Show optimiser diff</button>'
       + hint
       + '</div>';
}

function setupDisasmToolbar(pane, paneId, kind, optAvailable) {
  var toggle = pane.querySelector('[data-disasm-opt-toggle]');
  if (toggle) {
    toggle.addEventListener('change', function() {
      wasmOptState[paneId] = toggle.checked;
      renderDisassembly(paneId, kind);
    });
  }
  var diffBtn = pane.querySelector('[data-disasm-diff]');
  if (diffBtn && optAvailable) {
    diffBtn.addEventListener('click', function() {
      openOptDiffView(paneId, kind);
    });
  }
}

function renderDisasmEntry(entry, kind) {
  if (entry.kind === 'module') return renderWasmModuleHeader(entry);
  return renderDisasmFunction(entry, kind);
}

function renderDisasmFunction(entry, kind) {
  var sr = entry.sourceRange;
  var headerAttrs = sr ? ' data-start="' + sr.startOffset + '" data-end="' + sr.endOffset + '"' : '';
  var html = '<div class="wasm-function" data-kind="' + esc(kind) + '" data-func-idx="' + (entry.funcIdx !== undefined ? entry.funcIdx : '') + '" data-func-name="' + esc(entry.name) + '">';
  var kindBadge = '';
  if (entry.kind === 'top') kindBadge = ' <span class="wasm-kind-badge">top</span>';
  else if (entry.kind === 'method') kindBadge = ' <span class="wasm-kind-badge">method</span>';
  var sig = '';
  if (kind === 'wasm') {
    var params = (entry.params || []).map(function(p) { return '(param ' + esc(p.name) + ' ' + esc(p.type) + ')'; }).join(' ');
    var results = (entry.results || []).map(function(r) { return '(result ' + esc(r) + ')'; }).join(' ');
    sig = '(func <span class="wasm-func-name">$' + esc(entry.name) + '</span>' + kindBadge + (params ? ' ' + params : '') + (results ? ' ' + results : '') + ')';
  } else {
    sig = 'ByteCode <span class="wasm-func-name">' + esc(entry.name) + '</span>' + kindBadge;
  }
  var meta = '';
  if (kind === 'asm') {
    meta = ' <span class="wasm-comment">; ' + entry.instrCount + ' instructions, ' + (entry.byteCount || 0) + ' bytes, ' + ((entry.literals || []).length) + ' literals, ' + ((entry.locals || []).length) + ' locals</span>';
  } else {
    meta = ' <span class="wasm-comment">; ' + entry.instrCount + ' instructions</span>';
  }
  html += '<div class="wasm-func-header"' + headerAttrs + '>' + sig + meta + '</div>';
  if (kind === 'wasm' && entry.locals && entry.locals.length) {
    html += '<div class="wasm-func-locals">';
    for (var loc of entry.locals) html += '<div class="wasm-local">(local ' + esc(loc.name) + ' ' + esc(loc.type) + ')</div>';
    html += '</div>';
  } else if (kind === 'asm' && ((entry.literals || []).length || (entry.locals || []).length)) {
    html += '<details class="disasm-tables"><summary>literals &amp; locals</summary>';
    if ((entry.literals || []).length) {
      html += '<div class="disasm-tables-section">Literals:</div>';
      for (var i = 0; i < entry.literals.length; i++) {
        html += '<div class="wasm-local">' + i + ': "' + esc(entry.literals[i]) + '"</div>';
      }
    }
    if ((entry.locals || []).length) {
      html += '<div class="disasm-tables-section">Local variables:</div>';
      for (var i = 0; i < entry.locals.length; i++) {
        html += '<div class="wasm-local">%v' + i + ': "' + esc(entry.locals[i]) + '"</div>';
      }
    }
    html += '</details>';
  }
  // Edges are pre-laid-out by the backend (cfg_layout.assign_lanes via
  // tooling/cli/serialise._wasm_edges_with_lanes).  Stash the list on
  // the container so drawWasmEdges renders the lanes the backend
  // assigned — no client-side computation.
  var edgesJson = entry.edges ? JSON.stringify(entry.edges) : '[]';
  html += '<div class="wasm-func-body wasm-edges-container" data-func-idx="' + (entry.funcIdx !== undefined ? entry.funcIdx : '') + '" data-edges="' + esc(edgesJson) + '">';
  var source = getSource();
  var prevRangeKey = null;
  for (var ins of (entry.instructions || [])) {
    if (ins.kind === 'label') {
      html += renderDisasmLabel(ins);
      continue;
    }
    var r = ins.range;
    var rkey = r ? r.startOffset + '/' + r.endOffset : null;
    if (rkey && rkey !== prevRangeKey) {
      var snippet = wasmSourceSnippet(source, r);
      html += '<div class="wasm-src-comment" data-start="' + r.startOffset + '" data-end="' + r.endOffset + '">'
            + '<span class="wasm-gutter-pad"></span>'
            + '<span class="wasm-idx">' + (r.startLine + 1) + '</span>'
            + '<span class="wasm-src-text">; ' + esc(snippet) + '</span>'
            + '</div>';
      prevRangeKey = rkey;
    }
    if (kind === 'wasm') html += renderWasmInstruction(ins, entry);
    else html += renderAsmInstruction(ins, entry);
  }
  html += '</div></div>';
  return html;
}

function renderDisasmLabel(ins) {
  return '<div class="wasm-instr disasm-label-row" data-idx="' + ins.idx + '" data-label-name="' + esc(ins.label) + '">'
       + '<span class="wasm-gutter-pad"></span>'
       + '<span class="wasm-idx">' + ins.idx + '</span>'
       + '<span class="wasm-code"><span class="disasm-label-anchor"># ' + esc(ins.label) + ':</span></span>'
       + '</div>';
}

function renderAsmInstruction(ins, entry) {
  var range = ins.range;
  var rngAttrs = range ? ' data-start="' + range.startOffset + '" data-end="' + range.endOffset + '"' : '';
  var operandHtml = '';
  var jt = ins.jumpTarget;
  if (jt) {
    var btAttrs = ' data-branch-target-idx="' + (jt.targetIdx !== null && jt.targetIdx !== undefined ? jt.targetIdx : '') + '"'
                + ' data-branch-target-label="' + esc(jt.label) + '"';
    operandHtml = ' <span class="wasm-operand">' + esc(ins.operandText) + '</span>'
                + ' <span class="wasm-branch-target"' + btAttrs + '>; ' + esc(jt.label) + '</span>';
  } else if (ins.operandText) {
    operandHtml = ' <span class="wasm-operand">' + esc(ins.operandText) + '</span>';
  }
  var jumpTableHtml = '';
  if (ins.jumpTable && ins.jumpTable.length) {
    var jtEntries = [];
    for (var e of ins.jumpTable) {
      jtEntries.push('<span class="wasm-branch-target" data-branch-target-idx="' + (e.targetIdx !== null && e.targetIdx !== undefined ? e.targetIdx : '') + '" data-branch-target-label="' + esc(e.label) + '">&quot;' + esc(e.pattern) + '&quot;-&gt;' + esc(e.label) + '</span>');
    }
    jumpTableHtml = ' <span class="wasm-comment">[' + jtEntries.join(', ') + ']</span>';
  }
  var comment = ins.comment ? ' <span class="wasm-comment">; ' + esc(ins.comment) + '</span>' : '';
  return '<div class="wasm-instr" data-idx="' + ins.idx + '"' + rngAttrs + '>'
       + '<span class="wasm-gutter-pad"></span>'
       + '<span class="wasm-idx">(' + ins.offset + ')</span>'
       + '<span class="wasm-code">'
       + '<span class="wasm-mnemonic">' + esc(ins.op) + '</span>'
       + operandHtml
       + jumpTableHtml
       + comment
       + '</span></div>';
}

function renderWasmModuleHeader(entry) {
  var html = '<div class="wasm-module">';
  html += '<div class="wasm-func-header">(module) <span class="wasm-comment">' + entry.imports.length + ' imports, ' + entry.types.length + ' types, ' + entry.dataSegments.length + ' data segments</span></div>';
  if (entry.imports.length) {
    html += '<details class="wasm-module-details"><summary>imports (' + entry.imports.length + ')</summary>';
    for (var imp of entry.imports) {
      html += '<div class="wasm-import-entry">';
      html += '<span class="wasm-idx">' + imp.funcIdx + '</span>';
      html += '<span class="wasm-mnemonic">import</span> ';
      html += '<span class="wasm-operand">&quot;' + esc(imp.module) + '&quot;.&quot;' + esc(imp.name) + '&quot;</span> ';
      html += '<span class="wasm-comment">; type $t' + imp.typeIdx + '</span>';
      html += '</div>';
    }
    html += '</details>';
  }
  if (entry.dataSegments.length) {
    html += '<details class="wasm-module-details"><summary>data segments (' + entry.dataSegments.length + ')</summary>';
    for (var seg of entry.dataSegments) {
      html += '<div class="wasm-import-entry"><span class="wasm-idx">' + seg.offset + '</span><span class="wasm-comment">; ' + seg.size + ' bytes</span></div>';
    }
    html += '</details>';
  }
  html += '</div>';
  return html;
}

function wasmSourceSnippet(source, range) {
  // Grab the entire source line that contains range.startOffset, then
  // trim to at most 160 characters for the comment.
  var nl = source.indexOf('\n', range.startOffset);
  var lineEnd = nl === -1 ? source.length : nl;
  // Find start of line
  var lineStart = source.lastIndexOf('\n', range.startOffset - 1) + 1;
  var line = source.substring(lineStart, lineEnd).replace(/^\s+/, '');
  if (line.length > 160) line = line.substring(0, 157) + '...';
  return line;
}

function renderWasmInstruction(ins, entry) {
  var range = ins.range;
  var rngAttrs = range ? ' data-start="' + range.startOffset + '" data-end="' + range.endOffset + '"' : '';
  var indentSpaces = '    '.repeat(Math.max(0, ins.indent));
  var operandHtml = '';
  if (ins.callTarget) {
    var ct = ins.callTarget;
    var ctLabel = ct.kind === 'import' ? ('import ' + ct.name) : ct.name;
    var ctAttrs = ' data-call-target-def-idx="' + (ct.defIdx !== null && ct.defIdx !== undefined ? ct.defIdx : '') + '"'
                + ' data-call-target-kind="' + esc(ct.kind) + '"'
                + ' data-call-target-name="' + esc(ct.name) + '"';
    operandHtml = ' <span class="wasm-operand">' + esc(ins.operandText) + '</span>'
                + ' <span class="wasm-call-target"' + ctAttrs + '>; ' + esc(ctLabel) + '</span>';
  } else if (ins.branchTarget) {
    var bt = ins.branchTarget;
    var btAttrs = ' data-branch-target-idx="' + (bt.targetIdx !== null && bt.targetIdx !== undefined ? bt.targetIdx : '') + '"';
    var btLabel = bt.kind || '';
    if (bt.label) btLabel += ' ' + bt.label;
    operandHtml = ' <span class="wasm-operand">' + esc(ins.operandText) + '</span>'
                + ' <span class="wasm-branch-target"' + btAttrs + '>; ' + esc(btLabel) + '</span>';
  } else if (ins.op === 'local.get' || ins.op === 'local.set' || ins.op === 'local.tee') {
    var parts = ins.fullText.split(' ');
    operandHtml = ' <span class="wasm-operand">' + esc(ins.operandText) + '</span>';
    if (parts.length > 2) operandHtml += ' <span class="wasm-operand-name">' + esc(parts.slice(2).join(' ')) + '</span>';
  } else if (ins.operandText) {
    operandHtml = ' <span class="wasm-operand">' + esc(ins.operandText) + '</span>';
  }
  var label = ins.label ? ' <span class="wasm-comment">;; ' + esc(ins.label) + '</span>' : '';
  var blockLbl = ins.blockLabel ? ' <span class="wasm-block-label">' + esc(ins.blockLabel) + '</span>' : '';
  var gutter = '<span class="wasm-gutter-pad"></span>';
  return '<div class="wasm-instr" data-idx="' + ins.idx + '" data-func-idx="' + entry.funcIdx + '"' + rngAttrs + '>'
       + gutter
       + '<span class="wasm-idx">' + ins.idx + '</span>'
       + '<span class="wasm-code"><span class="wasm-indent">' + indentSpaces + '</span>'
       + '<span class="wasm-mnemonic">' + esc(ins.op) + '</span>'
       + blockLbl
       + operandHtml
       + label
       + '</span></div>';
}

function setupDisasmInteractions(pane, funcs, kind) {
  // Stash the current function-entry list on the pane so the
  // listeners below always see the latest data.  ``renderDisassembly``
  // replaces ``pane.innerHTML`` on every re-render (e.g. opt-toggle,
  // diff-close) but keeps the pane element itself, so we guard the
  // listener attachment and let the handlers read live state from
  // ``pane._disasmFuncEntries`` instead of closing over the stale
  // value from the first render.  Without this guard every toggle
  // stacked a new trio of listeners — harmless today because the
  // handlers re-query the DOM by ``data-*`` attributes, but fragile
  // under future refactors.
  pane._disasmFuncEntries = funcs.filter(function(f) { return f.kind !== 'module'; });
  if (pane._disasmInteractionsWired) return;
  pane._disasmInteractionsWired = true;

  pane.addEventListener('click', function(e) {
    var entries = pane._disasmFuncEntries || [];
    var ct = e.target.closest('.wasm-call-target');
    if (ct) {
      e.stopImmediatePropagation();
      e.stopPropagation();
      var defIdxStr = ct.dataset.callTargetDefIdx;
      if (defIdxStr !== '') {
        var defIdx = parseInt(defIdxStr);
        if (!isNaN(defIdx) && defIdx >= 0 && defIdx < entries.length) {
          navigateToWasmFunction(pane, entries[defIdx]);
        }
      }
      return;
    }
    var bt = e.target.closest('.wasm-branch-target');
    if (bt) {
      e.stopImmediatePropagation();
      e.stopPropagation();
      var targetIdxStr = bt.dataset.branchTargetIdx;
      if (targetIdxStr !== '') {
        var targetIdx = parseInt(targetIdxStr);
        if (!isNaN(targetIdx)) {
          navigateToWasmInstruction(bt.closest('.wasm-function'), targetIdx);
        }
      }
      return;
    }
  });
  // Hover highlights matching branch arrows and block/end pairs.
  pane.addEventListener('mouseover', function(e) {
    var ins = e.target.closest('.wasm-instr');
    if (!ins) return;
    var idx = ins.dataset.idx;
    var funcEl = ins.closest('.wasm-function');
    if (!funcEl) return;
    funcEl.querySelectorAll('.wasm-edge').forEach(function(p) {
      if (p.dataset.from === idx || p.dataset.to === idx) p.classList.add('highlighted');
    });
  });
  pane.addEventListener('mouseout', function(e) {
    var ins = e.target.closest('.wasm-instr');
    if (!ins) return;
    var funcEl = ins.closest('.wasm-function');
    if (!funcEl) return;
    funcEl.querySelectorAll('.wasm-edge.highlighted').forEach(function(p) { p.classList.remove('highlighted'); });
  });
  // Delegate the normal [data-start] click-to-source behaviour to the
  // consumer-supplied hook; registered second so the call/branch
  // short-circuits above can intercept first.
  setupHoverHighlighting(pane);
}

// Back-compat alias — the old name is referenced elsewhere in this
// file and may be inlined by external consumers.
var setupWasmInteractions = setupDisasmInteractions;

// Source-range highlight abstraction: standalone index.html defines
// ``highlightSourceRanges`` for inline highlighting; the VS Code
// webview uses postMessage.  We probe both so the WASM pane works in
// either consumer.
function wasmHighlightSource(start, end) {
  if (typeof highlightSourceRanges === 'function') {
    highlightSourceRanges([{start: start, end: end}]);
    return;
  }
  if (typeof vscode !== 'undefined' && vscode.postMessage) {
    vscode.postMessage({ type: 'highlightSource', start: start, end: end });
  }
}

function navigateToWasmFunction(pane, targetEntry) {
  if (!targetEntry) return;
  // ``CSS.escape`` is the only correct way to quote a dynamic value
  // inside an attribute selector (it handles every CSS special
  // character, not just ``\`` and ``"``).  The manual regex
  // fallback is kept for ancient environments that predate CSS.escape.
  var nameSel = (typeof CSS !== 'undefined' && typeof CSS.escape === 'function')
    ? CSS.escape(targetEntry.name)
    : targetEntry.name.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  var el = pane.querySelector('.wasm-function[data-func-name="' + nameSel + '"]');
  if (el) {
    el.scrollIntoView({ block: 'start', behavior: 'smooth' });
    // Flash-highlight the function header briefly.
    var hdr = el.querySelector('.wasm-func-header');
    if (hdr) {
      hdr.classList.add('wasm-func-flash');
      setTimeout(function() { hdr.classList.remove('wasm-func-flash'); }, 1200);
    }
    if (targetEntry.sourceRange) {
      wasmHighlightSource(targetEntry.sourceRange.startOffset, targetEntry.sourceRange.endOffset);
    }
  }
}

function navigateToWasmInstruction(funcEl, targetIdx) {
  if (!funcEl) return;
  var el = funcEl.querySelector('.wasm-instr[data-idx="' + targetIdx + '"]');
  if (!el) return;
  el.scrollIntoView({ block: 'center', behavior: 'smooth' });
  el.classList.add('wasm-instr-flash');
  setTimeout(function() { el.classList.remove('wasm-instr-flash'); }, 1200);
  var start = parseInt(el.dataset.start);
  var end = parseInt(el.dataset.end);
  if (!isNaN(start) && !isNaN(end)) wasmHighlightSource(start, end);
}

// Draw orthogonal control-flow arrows in the gutter of each function.
//
// Edges (and their lane assignments) come from the backend via the
// container's ``data-edges`` attribute — set by renderDisasmFunction
// from ``entry.edges`` (tooling/cli/serialise._wasm_edges_with_lanes).
// The renderer just maps idx → DOM element and lets drawOrthogonalEdges
// paint the lanes.  No client-side branch-target scanning or layout.
function drawWasmEdges(container) {
  var instrs = Array.from(container.querySelectorAll('.wasm-instr'));
  if (!instrs.length) return;
  var byIdx = {};
  for (var el of instrs) byIdx[el.dataset.idx] = el;
  var rawEdges = [];
  try {
    rawEdges = JSON.parse(container.dataset.edges || '[]');
  } catch (_) {
    rawEdges = [];
  }
  var edges = [];
  for (var e of rawEdges) {
    var fromEl = byIdx[String(e.fromIdx)];
    var toEl = byIdx[String(e.toIdx)];
    if (!fromEl || !toEl) continue;
    edges.push({
      from: fromEl,
      to: toEl,
      fromId: String(e.fromIdx),
      toId: String(e.toIdx),
      fromPos: e.fromIdx,
      toPos: e.toIdx,
      kind: e.kind,
      lane: e.lane,
    });
  }
  drawOrthogonalEdges(container, edges, {
    svgClass: 'wasm-edges-svg',
    edgeClass: 'wasm-edge',
    edgeKindClass: function(e) { return 'wasm-edge-' + e.kind; },
    arrowheadClass: 'wasm-arrowhead',
    arrowheadIdPrefix: 'wasm-ah-' + ((container.dataset.funcIdx || '0').replace(/[^A-Za-z0-9]/g, '_')),
    markerKinds: ['default'],
    markerForEdge: function() { return 'default'; },
    gutter: { laneWidth: 6, innerX: 24, entryX: 28, exitX: 28, cornerRadius: 4 },
    endpointSelector: '.wasm-instr',
    endpointIdAttr: 'idx',
  });
}

// Shared orthogonal-edge renderer.  All four current edge views
// (CFG pre- and post-SSA, WASM/ASM disassembly, and the optimiser
// diff brackets) go through this helper so improvements to the
// drawing, lane-assignment, hover, and accessibility story apply
// uniformly.
//
// Contract:
//
//   drawOrthogonalEdges(container, edges, options)
//
// Inputs:
//   container  — the DOM element that owns the edge SVG.  Must be
//                position-relative so the absolutely-positioned SVG
//                lines up with the endpoints.
//   edges      — list of {from, to, fromId?, toId?, fromPos, toPos,
//                kind?, directed?} descriptors.  ``from`` / ``to``
//                are DOM elements whose bounding rects we anchor to.
//                ``fromPos`` / ``toPos`` are monotonic integers used
//                for lane assignment (smallest span gets innermost
//                lane).  ``kind`` controls the CSS modifier class
//                and the arrowhead picker.  ``directed`` (default
//                true) toggles the arrowhead at the ``to`` end.
//   options    — renderer knobs; see below.
//
// Options:
//   svgClass            — class to put on the generated <svg>
//                         element (also the "remove previous" key).
//   edgeClass           — common class on every edge <path>
//                         (default 'oe-edge').
//   edgeKindClass       — function(edge) → class suffix, or a string
//                         prefix appended with edge.kind.
//   arrowheadClass      — class on the arrowhead <polygon> inside
//                         each marker (default 'oe-arrowhead').
//   arrowheadIdPrefix   — unique prefix for arrowhead marker IDs so
//                         multiple SVGs on one page don't collide.
//   markerKinds         — list of arrowhead kinds to define.
//   markerForEdge       — function(edge) → kind name used as
//                         ``marker-end="url(#<prefix>-<kind>)"``.
//   gutter              — geometry: {laneWidth, innerX, entryX,
//                         exitX, cornerRadius}.
//   endpointSelector    — CSS selector for endpoint DOM elements
//                         (used to wire hover-highlighting between
//                         edges and their endpoints).
//   endpointIdAttr      — data- attribute on each endpoint whose
//                         value matches edge.fromId / edge.toId
//                         (e.g. "block", "idx", "opt-group").
//
// The rendered edges carry three stable data-* attributes:
//   data-edge-from  — edge.fromId (or edge.from's matching
//                     endpoint id, if fromId is not given).
//   data-edge-to    — edge.toId (as above).
//   data-edge-kind  — edge.kind.
//
// Hover wiring (enabled whenever ``endpointSelector`` is provided):
//   - Hovering an endpoint adds ``highlighted`` to every edge whose
//     from-id or to-id matches that endpoint's data id, and adds
//     ``oe-endpoint-highlight`` to the related endpoints.
//   - Hovering an edge path adds ``highlighted`` to the edge and
//     ``oe-endpoint-highlight`` to both endpoints.
function drawOrthogonalEdges(container, edges, options) {
  // Clean up any previous render so re-layouts don't stack SVGs.
  if (options.svgClass) {
    container.querySelectorAll('.' + options.svgClass).forEach(function(s) { s.remove(); });
  }
  if (!edges.length) return null;

  // Lanes always come from the backend — CFG edges carry lanes from
  // ``cfg_layout.build_cfg_edges``, WASM edges from the WASM serialiser
  // (``_wasm_edges_with_lanes``), and opt-diff brackets are all
  // single-point spans (lane 0).  Renderer never recomputes layout.
  for (var edge of edges) {
    if (edge.lane == null) edge.lane = 0;
  }

  var rect = container.getBoundingClientRect();
  var svgNs = 'http://www.w3.org/2000/svg';
  var svg = document.createElementNS(svgNs, 'svg');
  if (options.svgClass) svg.classList.add(options.svgClass);
  svg.classList.add('oe-svg');
  svg.setAttribute('width', rect.width);
  svg.setAttribute('height', rect.height);

  // Arrowhead markers (one per distinct kind).
  var markerPrefix = options.arrowheadIdPrefix || 'oe-ah';
  var markerKinds = options.markerKinds || ['default'];
  if (markerKinds.length) {
    var defs = document.createElementNS(svgNs, 'defs');
    for (var kind of markerKinds) {
      var marker = document.createElementNS(svgNs, 'marker');
      marker.setAttribute('id', markerPrefix + '-' + kind);
      marker.setAttribute('viewBox', '0 0 8 6');
      marker.setAttribute('refX', '8');
      marker.setAttribute('refY', '3');
      marker.setAttribute('markerWidth', '7');
      marker.setAttribute('markerHeight', '5');
      marker.setAttribute('orient', 'auto');
      var poly = document.createElementNS(svgNs, 'polygon');
      poly.setAttribute('points', '0 0, 8 3, 0 6');
      if (options.arrowheadClass) {
        poly.classList.add(options.arrowheadClass);
        poly.classList.add(options.arrowheadClass + '-' + kind);
      }
      marker.appendChild(poly);
      defs.appendChild(marker);
    }
    svg.appendChild(defs);
  }

  var g = options.gutter || {};
  var laneW = g.laneWidth != null ? g.laneWidth : 8;
  var innerX = g.innerX != null ? g.innerX : 24;
  var entryX = g.entryX != null ? g.entryX : 0;
  var exitX = g.exitX != null ? g.exitX : 0;
  var R = g.cornerRadius != null ? g.cornerRadius : 4;
  var minX = g.minX != null ? g.minX : 4;

  for (var edge of edges) {
    var fromRect = edge.from.getBoundingClientRect();
    var toRect = edge.to.getBoundingClientRect();
    var y1 = g.anchorFromY === 'bottom'
      ? fromRect.bottom - rect.top - 4
      : (g.anchorFromY === 'top' ? fromRect.top - rect.top + 4 : fromRect.top - rect.top + fromRect.height / 2);
    var y2 = g.anchorToY === 'top'
      ? toRect.top - rect.top + 8
      : (g.anchorToY === 'bottom' ? toRect.bottom - rect.top - 4 : toRect.top - rect.top + toRect.height / 2);
    var laneX = innerX - edge.lane * laneW;
    if (laneX < minX) laneX = minX;
    // Compute from/to X anchors: CFG edges anchor to each block's
    // left edge (so the gutter is outside the block); disassembly
    // edges anchor to a fixed exit column.
    var xFrom, xTo;
    if (g.anchorFromX === 'left-of') {
      xFrom = fromRect.left - rect.left;
    } else {
      xFrom = entryX;
    }
    if (g.anchorToX === 'left-of') {
      xTo = toRect.left - rect.left;
    } else {
      xTo = exitX;
    }
    var goingDown = y2 >= y1;
    var d;
    if (Math.abs(y2 - y1) < 2) {
      d = 'M ' + xFrom + ' ' + y1 + ' L ' + xTo + ' ' + y2;
    } else {
      var dy = goingDown ? 1 : -1;
      d = 'M ' + xFrom + ' ' + y1
        + ' L ' + (laneX + R) + ' ' + y1
        + ' A ' + R + ' ' + R + ' 0 0 ' + (goingDown ? 1 : 0) + ' ' + laneX + ' ' + (y1 + dy * R)
        + ' L ' + laneX + ' ' + (y2 - dy * R)
        + ' A ' + R + ' ' + R + ' 0 0 ' + (goingDown ? 0 : 1) + ' ' + (laneX + R) + ' ' + y2
        + ' L ' + xTo + ' ' + y2;
    }
    var path = document.createElementNS(svgNs, 'path');
    path.setAttribute('d', d);
    path.classList.add('oe-edge');
    if (options.edgeClass) path.classList.add(options.edgeClass);
    var kindClass = typeof options.edgeKindClass === 'function' ? options.edgeKindClass(edge) : (options.edgeKindClass && edge.kind ? options.edgeKindClass + edge.kind : '');
    if (kindClass) path.classList.add(kindClass);
    if (edge.fromId != null) path.dataset.edgeFrom = edge.fromId;
    if (edge.toId != null) path.dataset.edgeTo = edge.toId;
    if (edge.kind != null) path.dataset.edgeKind = edge.kind;
    // Preserve legacy data attrs that older CSS and hover handlers
    // keyed off of, so existing styles keep working.
    if (edge.fromId != null) path.dataset.from = edge.fromId;
    if (edge.toId != null) path.dataset.to = edge.toId;
    if (edge.directed !== false) {
      var marker = options.markerForEdge ? options.markerForEdge(edge) : 'default';
      if (marker) path.setAttribute('marker-end', 'url(#' + markerPrefix + '-' + marker + ')');
    }
    svg.appendChild(path);
  }
  container.insertBefore(svg, container.firstChild);

  // Hover wiring — happens once per container.  Re-renders replace
  // the SVG but keep the container, so we only install the listener
  // on first mount.
  if (options.endpointSelector && !container._oeHoverWired) {
    container._oeHoverWired = true;
    container.addEventListener('mouseover', function(e) {
      // 1) hover a path → highlight endpoints
      var path = e.target.closest && e.target.closest('.oe-edge');
      if (path) {
        path.classList.add('highlighted');
        highlightOrthogonalEndpoints(container, options, path.dataset.edgeFrom, path.dataset.edgeTo);
        return;
      }
      // 2) hover an endpoint → highlight related edges + paired endpoints
      var endpoint = e.target.closest(options.endpointSelector);
      if (!endpoint) return;
      var id = endpoint.dataset[options.endpointIdAttr];
      if (id == null) return;
      container.querySelectorAll('.oe-edge').forEach(function(p) {
        if (p.dataset.edgeFrom === id || p.dataset.edgeTo === id) {
          p.classList.add('highlighted');
        }
      });
      container.querySelectorAll('.oe-edge.highlighted').forEach(function(p) {
        highlightOrthogonalEndpoints(container, options, p.dataset.edgeFrom, p.dataset.edgeTo);
      });
    });
    container.addEventListener('mouseout', function(e) {
      var path = e.target.closest && e.target.closest('.oe-edge');
      if (path) {
        path.classList.remove('highlighted');
        clearOrthogonalEndpointHighlights(container);
        return;
      }
      var endpoint = e.target.closest(options.endpointSelector);
      if (!endpoint) return;
      container.querySelectorAll('.oe-edge.highlighted').forEach(function(p) { p.classList.remove('highlighted'); });
      clearOrthogonalEndpointHighlights(container);
    });
  }
  return svg;
}

function highlightOrthogonalEndpoints(container, options, fromId, toId) {
  if (!options.endpointSelector) return;
  var idAttr = options.endpointIdAttr;
  container.querySelectorAll(options.endpointSelector).forEach(function(el) {
    var id = el.dataset[idAttr];
    if (id != null && (id === fromId || id === toId)) {
      el.classList.add('oe-endpoint-highlight');
    }
  });
}

function clearOrthogonalEndpointHighlights(container) {
  container.querySelectorAll('.oe-endpoint-highlight').forEach(function(el) {
    el.classList.remove('oe-endpoint-highlight');
  });
}

// Opt diff — semantic-change-only comparison between the original and
// optimised disassembly.  The diff ignores instruction sequence
// numbers, byte offsets, literal/local indices (comparing by literal
// text and local name instead), and ``+N``/``pc N`` jump relatives
// (comparing by label name instead).  Changes to named targets
// (labels, call targets, literals, locals) surface as red/green rows.

function openOptDiffView(paneId, kind) {
  var pane = $('#' + paneId);
  if (!pane) return;
  var orig = kind === 'asm' ? data.asm : data.wasm;
  var opt = kind === 'asm' ? data.asmOptimised : data.wasmOptimised;
  if (!orig || !opt) return;
  // Match entries by name; render one diff block per function that
  // appears in either side.
  var byName = {};
  for (var e of orig) { if (e.kind !== 'module') byName[e.name] = { orig: e }; }
  for (var e of opt) {
    if (e.kind === 'module') continue;
    if (!byName[e.name]) byName[e.name] = {};
    byName[e.name].opt = e;
  }
  var ordered = [];
  for (var e of orig) { if (e.kind !== 'module') ordered.push(e.name); }
  for (var e of opt) {
    if (e.kind !== 'module' && ordered.indexOf(e.name) < 0) ordered.push(e.name);
  }
  var html = renderDisasmToolbar(paneId, kind, true, wasmOptState[paneId] === true);
  html += '<div class="disasm-diff-header">';
  html += '<div class="disasm-diff-title">Optimiser diff &mdash; original vs optimised</div>';
  html += '<div class="disasm-diff-legend">'
       + '<span class="disasm-diff-legend-removed">&minus; removed</span> '
       + '<span class="disasm-diff-legend-added">+ added</span> '
       + '<span class="disasm-diff-legend-dim">sequence numbers &amp; offsets ignored</span>'
       + '</div>';
  html += '<button class="disasm-diff-close" data-disasm-diff-close>Close diff</button>';
  html += '</div><div class="disasm-body disasm-diff-body">';
  var anyDiff = false;
  for (var name of ordered) {
    var pair = byName[name];
    var block = buildOptDiffBlockHtml(name, pair.orig, pair.opt, kind);
    if (block.changed) anyDiff = true;
    html += block.html;
  }
  if (!anyDiff) {
    html += '<div class="empty-state">No semantic changes detected (sequence numbers and offsets ignored).</div>';
  }
  html += '</div>';
  pane.innerHTML = html;
  setupDisasmToolbar(pane, paneId, kind, true);
  var closeBtn = pane.querySelector('[data-disasm-diff-close]');
  if (closeBtn) closeBtn.addEventListener('click', function() { renderDisassembly(paneId, kind); });
  setupHoverHighlighting(pane);
  // Draw control-flow arrows on the green (new) code in each diff
  // block — only edges whose source and target are both visible
  // opt-side rows get rendered, so same-segment elisions don't
  // produce dangling arrows.
  requestAnimationFrame(function() {
    pane.querySelectorAll('.disasm-diff-rows.wasm-edges-container').forEach(function(c) {
      drawWasmEdges(c);
    });
  });
}

function buildOptDiffBlockHtml(name, origEntry, optEntry, kind) {
  var origRows = origEntry ? normaliseForDiff(origEntry, kind) : [];
  var optRows = optEntry ? normaliseForDiff(optEntry, kind) : [];
  var segments = computeDiffSegments(origRows.map(function(r) { return r.key; }), optRows.map(function(r) { return r.key; }));
  var changed = segments.some(function(s) { return s.type !== 'same'; });
  // Visible opt-side rows contribute to the control-flow arrow
  // overlay (drawn alongside the green/new code).  Track which opt
  // instruction indices are shown so we can wire edges to them.
  var visibleOptIdx = new Set();
  var html = '<div class="wasm-function disasm-diff-block" data-func-name="' + esc(name) + '">';
  var badge = !origEntry ? '<span class="disasm-diff-badge disasm-diff-added">new</span>'
             : !optEntry ? '<span class="disasm-diff-badge disasm-diff-removed">removed</span>'
             : !changed ? '<span class="disasm-diff-badge disasm-diff-same">unchanged</span>'
             : '<span class="disasm-diff-badge disasm-diff-modified">modified</span>';
  html += '<div class="wasm-func-header">' + esc(name) + ' ' + badge + '</div>';
  if (!changed && origEntry) {
    html += '<div class="disasm-diff-unchanged-note">' + origRows.length + ' instructions — no semantic changes</div>';
    html += '</div>';
    return { html: html, changed: false };
  }
  // Wrap the rows in a ``wasm-edges-container`` so the shared
  // orthogonal-edge renderer can target it directly, keyed by the
  // ``data-idx`` stamped onto each visible opt-side row.
  html += '<div class="disasm-diff-rows wasm-edges-container" data-diff-func-name="' + esc(name) + '">';
  for (var seg of segments) {
    if (seg.type === 'same') {
      // For same segments show the opt-side row (not the orig side)
      // so ``data-idx`` stays in the opt instruction space and
      // arrows land correctly.
      var shown = Math.min(seg.optEnd - seg.optStart, 1);
      for (var i = 0; i < shown; i++) {
        var row = optRows[seg.optStart + i];
        if (row && row.instrIdx != null) visibleOptIdx.add(row.instrIdx);
        html += renderDiffRow(row, 'same', 'opt');
      }
      var hiddenCount = (seg.optEnd - seg.optStart) - shown;
      if (hiddenCount > 0) {
        html += '<div class="disasm-diff-elide">&hellip; ' + hiddenCount + ' unchanged instruction' + (hiddenCount === 1 ? '' : 's') + '</div>';
      }
    } else {
      for (var i = seg.origStart; i < seg.origEnd; i++) html += renderDiffRow(origRows[i], 'removed', 'orig');
      for (var i = seg.optStart; i < seg.optEnd; i++) {
        var row = optRows[i];
        if (row && row.instrIdx != null) visibleOptIdx.add(row.instrIdx);
        html += renderDiffRow(row, 'added', 'opt');
      }
    }
  }
  html += '</div></div>';
  return { html: html, changed: changed, visibleOptIdx: visibleOptIdx };
}

function renderDiffRow(row, state, side) {
  if (!row) return '';
  var cls = 'disasm-diff-row disasm-diff-' + state;
  // Added/same rows double as control-flow arrow endpoints via the
  // same ``wasm-instr`` / ``data-idx`` contract that the live
  // disassembly uses.  Removed rows are just text.
  if (side === 'opt' && row.instrIdx != null) cls += ' wasm-instr';
  var sigil = state === 'added' ? '+' : (state === 'removed' ? '−' : ' ');
  var rng = row.range;
  var rngAttrs = rng ? ' data-start="' + rng.startOffset + '" data-end="' + rng.endOffset + '"' : '';
  var idxAttr = (side === 'opt' && row.instrIdx != null) ? ' data-idx="' + row.instrIdx + '"' : '';
  // The normalised display already carries a ``.wasm-branch-target``
  // span for branching instructions; for opt-side rows we thread
  // ``data-branch-target-idx`` onto it so ``drawWasmEdges`` can
  // find the endpoint from inside the diff container.
  var displayHtml = row.displayHtml;
  if (side === 'opt' && row.branchTargetIdx != null) {
    displayHtml = displayHtml.replace(
      /<span class="wasm-branch-target"/,
      '<span class="wasm-branch-target" data-branch-target-idx="' + row.branchTargetIdx + '"'
    );
  }
  return '<div class="' + cls + '"' + idxAttr + rngAttrs + '>'
       + '<span class="disasm-diff-sigil">' + sigil + '</span>'
       + '<span class="disasm-diff-text">' + displayHtml + '</span>'
       + '</div>';
}

function normaliseForDiff(entry, kind) {
  // Map each instruction (or label) into a ``{key, displayHtml,
  // range}`` tuple where ``key`` is a string uniquely identifying the
  // semantic content — explicitly excluding sequence numbers, byte
  // offsets, and ``+N``/``pc N`` jump relatives.  The frontend diff
  // engine (``computeDiffSegments``) compares keys; rows with equal
  // keys are grouped into ``same`` segments regardless of where they
  // sit in the stream.
  var rows = [];
  var literals = entry.literals || [];
  var locals = entry.locals || [];
  for (var ins of (entry.instructions || [])) {
    if (ins.kind === 'label') {
      rows.push({
        key: 'LABEL:' + ins.label,
        displayHtml: '<span class="disasm-label-anchor"># ' + esc(ins.label) + ':</span>',
        range: null,
        instrIdx: ins.idx,
        branchTargetIdx: null,
      });
      continue;
    }
    var parts = [ins.op];
    var displayParts = ['<span class="wasm-mnemonic">' + esc(ins.op) + '</span>'];
    if (kind === 'asm') {
      var jt = ins.jumpTarget;
      if (jt) {
        parts.push('JMP:' + jt.label);
        displayParts.push('<span class="wasm-branch-target">; ' + esc(jt.label) + '</span>');
      } else if (ins.operandText) {
        // Preserve literal/local refs as names, but strip bare numbers
        // (they're indices whose identity depends on table order).
        var norm = normaliseAsmOperand(ins, literals, locals);
        parts.push(norm.key);
        displayParts.push('<span class="wasm-operand">' + esc(norm.display) + '</span>');
      }
      if (ins.jumpTable && ins.jumpTable.length) {
        var jtKeys = ins.jumpTable.map(function(e) { return e.pattern + '->' + e.label; });
        parts.push('JT:' + jtKeys.join('|'));
        var jtDisplay = ins.jumpTable.map(function(e) { return '&quot;' + esc(e.pattern) + '&quot;->' + esc(e.label); });
        displayParts.push('<span class="wasm-comment">[' + jtDisplay.join(', ') + ']</span>');
      }
    } else {
      // WASM operands: resolve call/branch by target name.
      if (ins.callTarget) {
        var ct = ins.callTarget;
        parts.push('CALL:' + ct.kind + ':' + ct.name);
        displayParts.push('<span class="wasm-call-target">; ' + esc(ct.kind === 'import' ? ('import ' + ct.name) : ct.name) + '</span>');
      } else if (ins.branchTarget) {
        var bt = ins.branchTarget;
        parts.push('BR:' + (bt.kind || '') + ':' + (bt.label || ''));
        var lbl = (bt.kind || '') + (bt.label ? ' ' + bt.label : '');
        displayParts.push('<span class="wasm-branch-target">; ' + esc(lbl) + '</span>');
      } else if (ins.op === 'local.get' || ins.op === 'local.set' || ins.op === 'local.tee') {
        var localParts = ins.fullText.split(' ');
        var localName = localParts.length > 2 ? localParts.slice(2).join(' ') : '';
        parts.push('LOCAL:' + (localName || ins.operandText));
        displayParts.push('<span class="wasm-operand-name">' + esc(localName || ins.operandText) + '</span>');
      } else if (ins.op === 'i32.const' || ins.op === 'i64.const' || ins.op === 'f64.const') {
        // Constant value IS semantic — keep it.
        parts.push('K:' + ins.operandText);
        displayParts.push('<span class="wasm-operand">' + esc(ins.operandText) + '</span>');
      } else if (ins.operandText) {
        // Other ops: operand is typically a type descriptor or a
        // local/global idx.  Keep the text but strip bare numerics.
        var stripped = ins.operandText.replace(/\b\d+\b/g, '#');
        parts.push(stripped);
        if (stripped !== '#') displayParts.push('<span class="wasm-operand">' + esc(ins.operandText) + '</span>');
      }
      if (ins.label) {
        parts.push('LBL:' + ins.label);
        displayParts.push('<span class="wasm-comment">;; ' + esc(ins.label) + '</span>');
      }
    }
    // Record the stable instruction idx + resolved branch target so
    // the diff view can draw control-flow arrows on the green side.
    var branchTargetIdx = null;
    if (kind === 'asm' && ins.jumpTarget && ins.jumpTarget.targetIdx != null) {
      branchTargetIdx = ins.jumpTarget.targetIdx;
    } else if (kind === 'wasm' && ins.branchTarget && ins.branchTarget.targetIdx != null) {
      branchTargetIdx = ins.branchTarget.targetIdx;
    }
    rows.push({
      key: parts.join(' '),
      displayHtml: displayParts.join(' '),
      range: ins.range,
      instrIdx: ins.idx,
      branchTargetIdx: branchTargetIdx,
    });
  }
  return rows;
}

function normaliseAsmOperand(ins, literals, locals) {
  // Map push1/push4 N → literal text; loadScalar1/storeScalar1 %vN →
  // local name; jump/pc-anchored ops are handled via ``jumpTarget``.
  // Returns ``{key, display}``.
  var op = ins.op;
  var text = ins.operandText;
  if ((op === 'push1' || op === 'push4') && ins.operandText) {
    var idx = parseInt(ins.operandText);
    if (!isNaN(idx) && idx >= 0 && idx < literals.length) {
      return { key: 'LIT:' + literals[idx], display: '"' + literals[idx] + '" (#' + idx + ')' };
    }
  }
  if (ins.lvtRef !== null && ins.lvtRef !== undefined) {
    var name = locals[ins.lvtRef] || ('v' + ins.lvtRef);
    return { key: 'VAR:' + name, display: '%' + name };
  }
  // Strip bare "+N" / "-N" / "pc N" relatives from non-label ops.
  var stripped = text.replace(/[+-]?\d+/g, '#').replace(/pc #/g, 'pc');
  return { key: stripped, display: text };
}

// SSA variable spans with hover tooltips
function renderVarSpan(name,version,type,lattice,role){
  var cls=role==='def'?'ssa-var ssa-var-def':'ssa-var ssa-var-use';
  var ttData={name:name,version:version};if(type)ttData.type=type;if(lattice)ttData.lattice=lattice;ttData.role=role;
  var span='<span class="'+cls+'" data-var=\''+esc(JSON.stringify(ttData))+'\'>'+esc(name)+'#'+version+'</span>';
  if(role==='def'&&type)span+='<span class="ssa-var-type">:'+esc(type)+'</span>';
  return span;
}
function renderSSAInfo(uses,defs){
  var hasUses=Object.keys(uses).length>0;var hasDefs=Object.keys(defs).length>0;
  if(!hasUses&&!hasDefs)return'';
  var html='<div class="cfg-ssa-info">';
  if(hasUses){var useSpans=Object.entries(uses).map(function(e){var n=e[0],u=e[1];return typeof u==='object'?renderVarSpan(n,u.version,u.type||null,u.lattice||null,'use'):renderVarSpan(n,u,null,null,'use');});html+='uses={'+useSpans.join(', ')+'}';}
  if(hasUses&&hasDefs)html+=' ';
  if(hasDefs){var defSpans=Object.entries(defs).map(function(e){var n=e[0],d=e[1];return renderVarSpan(n,d.version,d.type||null,d.lattice||null,'def');});html+='defs={'+defSpans.join(', ')+'}';}
  html+='</div>';return html;
}

// Variable tooltip system
var activeTooltip=null;
function setupVarTooltips(container){
  container.addEventListener('mouseover',function(e){var el=e.target.closest('.ssa-var');if(!el)return;showVarTooltip(el,JSON.parse(el.dataset.var));});
  container.addEventListener('mouseout',function(e){var el=e.target.closest('.ssa-var');if(!el)return;hideVarTooltip();});
}
function showVarTooltip(el,v){
  hideVarTooltip();
  var tt=document.createElement('div');tt.className='var-tooltip';
  var html='<div class="tt-name">'+esc(v.name)+'#'+v.version+'</div><div class="tt-row"><span class="tt-label">role</span><span class="tt-val">'+v.role+'</span></div>';
  if(v.type)html+='<div class="tt-row"><span class="tt-label">type</span><span class="tt-type">'+esc(v.type)+'</span></div>';
  if(v.lattice)html+='<div class="tt-row"><span class="tt-label">value</span><span class="tt-lattice">'+esc(v.lattice)+'</span></div>';
  if(!v.type&&!v.lattice)html+='<div class="tt-row"><span class="tt-label">info</span><span class="tt-val" style="color:var(--text-dim)">no type/value inferred</span></div>';
  tt.innerHTML=html;document.body.appendChild(tt);activeTooltip=tt;
  var rect=el.getBoundingClientRect();tt.style.left=rect.left+'px';tt.style.top=(rect.bottom+6)+'px';
  var ttRect=tt.getBoundingClientRect();
  if(ttRect.right>window.innerWidth-8)tt.style.left=(window.innerWidth-ttRect.width-8)+'px';
  if(ttRect.bottom>window.innerHeight-8)tt.style.top=(rect.top-ttRect.height-6)+'px';
}
function hideVarTooltip(){if(activeTooltip){activeTooltip.remove();activeTooltip=null;}}

// CFG edge arrows
function drawAllCfgEdges(pane, funcs) {
  for (var func of funcs) {
    var container = pane.querySelector('.cfg-edges-container[data-func="' + func.name + '"]');
    if (!container) continue;
    var blockEls = {};
    container.querySelectorAll('.cfg-block[data-block]').forEach(function(el) { blockEls[el.dataset.block] = el; });
    // Edges + lanes come precomputed from the shared cfg_layout routing model
    // (serialised as func.edges), so the SVG router and the CLI/TUI ASCII
    // gutter nest control-flow edges identically.
    var edges = [];
    for (var e of (func.edges || [])) {
      var fromEl = blockEls[e.from];
      var toEl = blockEls[e.to];
      if (!fromEl || !toEl) continue;
      edges.push({
        from: fromEl,
        to: toEl,
        fromId: e.from,
        toId: e.to,
        fromPos: e.fromPos,
        toPos: e.toPos,
        kind: e.kind,
        lane: e.lane,
      });
    }
    var fid = func.name.replace(/[^A-Za-z0-9]/g, '_');
    drawOrthogonalEdges(container, edges, {
      svgClass: 'cfg-edges-svg',
      edgeClass: 'cfg-edge',
      edgeKindClass: function(e) { return 'cfg-edge-' + e.kind; },
      arrowheadClass: 'cfg-arrowhead',
      arrowheadIdPrefix: 'cfg-ah-' + fid,
      markerKinds: ['true', 'false', 'goto'],
      markerForEdge: function(e) { return e.kind; },
      gutter: {
        laneWidth: 8, innerX: 36, cornerRadius: 4,
        anchorFromX: 'left-of', anchorToX: 'left-of',
        anchorFromY: 'bottom', anchorToY: 'top',
      },
      endpointSelector: '.cfg-block[data-block]',
      endpointIdAttr: 'block',
    });
  }
}

// Redraw edges on resize
var edgeRedrawTimer=null;
function scheduleEdgeRedraw(){clearTimeout(edgeRedrawTimer);edgeRedrawTimer=setTimeout(function(){if(!data)return;var prePane=$('#pane-cfg-pre');if(prePane.classList.contains('active'))drawAllCfgEdges(prePane,data.cfgPreSsa);var postPane=$('#pane-cfg-post');if(postPane.classList.contains('active'))drawAllCfgEdges(postPane,data.cfgPostSsa);var optPane=$('#pane-opt');if(optPane.classList.contains('active')&&data.optimisedSource)drawOptBrackets(optPane);},100);}

// Selection for copy-to-Claude
var selectedItems = new Map();
var copyFab = null;

function getViewName(el) {
  var pane = el.closest('.tab-pane');
  if (!pane) return 'Unknown';
  var key = pane.id.replace('pane-', '');
  var tab = $('.tab[data-tab="' + key + '"]');
  return tab ? tab.textContent.trim() : pane.id;
}

function offsetToLineCol(offset) {
  var src = getSource();
  var line = 0, col = 0;
  for (var i = 0; i < offset && i < src.length; i++) {
    if (src[i] === '\n') { line++; col = 0; } else { col++; }
  }
  return { line: line + 1, col: col + 1 };
}

function extractItemData(el) {
  var start = parseInt(el.dataset.start);
  var end = parseInt(el.dataset.end);
  var startLC = offsetToLineCol(start);
  var endLC = offsetToLineCol(end);
  var view = getViewName(el);
  var summary = el.textContent.trim().replace(/\s+/g, ' ');
  var code = null;
  var codeEl = el.querySelector('.opt-code, .shimmer-code, .gvn-code, .taint-code, .irules-code');
  if (codeEl) code = codeEl.textContent.trim();
  var detail = null;
  var replEl = el.querySelector('.opt-repl');
  if (replEl) detail = replEl.textContent.trim();
  return { view: view, summary: summary, code: code, detail: detail, startOffset: start, endOffset: end, startLine: startLC.line, startCol: startLC.col, endLine: endLC.line, endCol: endLC.col };
}

function toggleSelection(el) {
  if (selectedItems.has(el)) { selectedItems.delete(el); el.classList.remove('selected'); }
  else { selectedItems.set(el, extractItemData(el)); el.classList.add('selected'); }
  updateCopyFab();
}

function clearSelection() {
  for (var el of selectedItems.keys()) { el.classList.remove('selected'); }
  selectedItems.clear();
  updateCopyFab();
}

function selectAllInActiveTab() {
  var activePane = document.querySelector('.tab-pane.active');
  if (!activePane) return;
  var selectables = activePane.querySelectorAll('[data-start]');
  for (var el of selectables) {
    if (!selectedItems.has(el)) { selectedItems.set(el, extractItemData(el)); el.classList.add('selected'); }
  }
  updateCopyFab();
}

function updateCopyFab() {
  var count = selectedItems.size;
  if (count === 0) { if (copyFab) { copyFab.remove(); copyFab = null; } return; }
  if (!copyFab) {
    copyFab = document.createElement('button');
    copyFab.className = 'copy-fab';
    copyFab.addEventListener('click', function() { copySelectionToClipboard(); });
    $('#outputPanel').appendChild(copyFab);
  }
  var key = isMac ? '\u2318C' : 'Ctrl+C';
  copyFab.innerHTML = 'Copy ' + count + ' item' + (count > 1 ? 's' : '') + ' <span class="key-hint">' + key + '</span>';
  copyFab.classList.remove('copied');
}

function buildClipboardMarkdown() {
  var src = getSource();
  var dialect = compiledDialect || $('#dialect').value;
  var items = Array.from(selectedItems.values());
  items.sort(function(a, b) { return a.startOffset - b.startOffset; });
  var groups = new Map();
  for (var item of items) { if (!groups.has(item.view)) groups.set(item.view, []); groups.get(item.view).push(item); }
  var minLine = Infinity, maxLine = -Infinity;
  for (var item of items) { minLine = Math.min(minLine, item.startLine); maxLine = Math.max(maxLine, item.endLine); }
  var srcLines = src.split('\n');
  var contextBefore = 2, contextAfter = 2;
  var fromLine = Math.max(1, minLine - contextBefore);
  var toLine = Math.min(srcLines.length, maxLine + contextAfter);
  var relevantSrc = srcLines.slice(fromLine - 1, toLine);
  var useFullSource = (toLine - fromLine + 1) >= srcLines.length * 0.7;
  var md = '## Compiler Explorer Selection\n\n';
  md += '### Source (' + dialect + ')\n';
  md += '```tcl\n';
  if (useFullSource) { md += src; }
  else {
    if (fromLine > 1) md += '# ... (lines 1-' + (fromLine - 1) + ' omitted)\n';
    for (var i = 0; i < relevantSrc.length; i++) { md += relevantSrc[i] + '\n'; }
    if (toLine < srcLines.length) md += '# ... (lines ' + (toLine + 1) + '-' + srcLines.length + ' omitted)\n';
  }
  if (!src.endsWith('\n')) md += '\n';
  md += '```\n\n';
  md += '### Selected items\n\n';
  for (var _pair of groups) {
    var view = _pair[0], viewItems = _pair[1];
    md += '**' + view + '** (' + viewItems.length + ' item' + (viewItems.length > 1 ? 's' : '') + ')\n';
    for (var item of viewItems) {
      var range = item.startLine + ':' + item.startCol + '\u2013' + item.endLine + ':' + item.endCol;
      var line = '- Line ' + range;
      if (item.code) line += ' [' + item.code + ']';
      line += ' `' + item.summary + '`';
      md += line + '\n';
    }
    md += '\n';
  }
  return md.trimEnd() + '\n';
}

async function copySelectionToClipboard() {
  if (selectedItems.size === 0) return;
  var md = buildClipboardMarkdown();
  try {
    await navigator.clipboard.writeText(md);
    if (copyFab) { copyFab.classList.add('copied'); copyFab.textContent = 'Copied!'; setTimeout(function() { updateCopyFab(); }, 1200); }
  } catch (err) {
    var ta = document.createElement('textarea');
    ta.value = md; ta.style.position = 'fixed'; ta.style.opacity = '0';
    document.body.appendChild(ta); ta.select(); document.execCommand('copy'); ta.remove();
    if (copyFab) { copyFab.classList.add('copied'); copyFab.textContent = 'Copied!'; setTimeout(function() { updateCopyFab(); }, 1200); }
  }
}
