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
function esc(s){if(s==null)return'';return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');}
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
  if (s.gvnWarnings) parts.push('gvn <span class="stat-value">' + s.gvnWarnings + '</span>');
  if (s.taintWarnings) parts.push('taint <span class="stat-value">' + s.taintWarnings + '</span>');
  if (s.irulesFlowWarnings) parts.push('irules <span class="stat-value">' + s.irulesFlowWarnings + '</span>');
  $('#stats').innerHTML = parts.join(' &middot; ');
}

// IR rendering
function renderIR() {
  var pane = $('#pane-ir');
  var html = '<div class="section-header">top-level</div><div class="ir-tree">';
  html += renderIRNodes(data.ir.topLevel);
  html += '</div>';
  var procs = data.ir.procedures;
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
  var html = '';
  for (var func of data.cfgPreSsa) {
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
  requestAnimationFrame(function() { drawAllCfgEdges(pane, data.cfgPreSsa); });
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
  var html = '';
  for (var func of data.cfgPostSsa) {
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
        html += '<div class="cfg-phi">' + renderVarSpan(phi.name, phi.version, phi.type, null, 'def') + ' \u2190 ' + esc(incoming) + (phi.type ? ' : ' + esc(phi.type) : '') + '</div>';
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
  requestAnimationFrame(function() { drawAllCfgEdges(pane, data.cfgPostSsa); });
}

// Interprocedural
function renderInterproc() {
  var pane = $('#pane-interproc');
  if (!data.interprocedural.length) { pane.innerHTML = '<div class="empty-state">No procedures to analyse</div>'; return; }
  var html = '';
  for (var p of data.interprocedural) {
    html += '<div class="proc-card">';
    html += '<div class="proc-name">' + esc(p.name) + ' <span style="color:var(--text-dim); font-size:11px">arity=' + esc(p.arity) + '</span>';
    html += '<span class="pure-badge ' + (p.pure ? 'pure-yes' : 'pure-no') + '">' + (p.pure ? 'pure' : 'impure') + '</span>';
    if (p.foldable) html += '<span class="pure-badge pure-yes">foldable</span>';
    html += '</div>';
    html += '<div class="proc-detail">return: <span class="val">' + esc(p.returnShape) + '</span></div>';
    html += '<div class="proc-detail">calls: <span class="val">' + (p.calls.length ? esc(p.calls.join(', ')) : '\u2014') + '</span></div>';
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
  var container=pane.querySelector('.opt-diff-container');
  if(!container)return;
  container.querySelectorAll('.opt-diff-svg').forEach(function(s){s.remove()});
  var groupIds=new Set();
  container.querySelectorAll('[data-opt-group]').forEach(function(el){groupIds.add(el.dataset.optGroup)});
  if(!groupIds.size)return;
  var containerRect=container.getBoundingClientRect();
  var svg=document.createElementNS('http://www.w3.org/2000/svg','svg');
  svg.classList.add('opt-diff-svg');
  svg.setAttribute('width','36');svg.setAttribute('height',container.offsetHeight);
  for(var gid of groupIds){
    var origEls=container.querySelectorAll('.opt-original[data-opt-group="'+gid+'"]');
    var replEls=container.querySelectorAll('.opt-replacement[data-opt-group="'+gid+'"]');
    if(!origEls.length||!replEls.length)continue;
    var firstOrigRect=origEls[0].getBoundingClientRect();
    var lastReplRect=replEls[replEls.length-1].getBoundingClientRect();
    var y1=firstOrigRect.top-containerRect.top+firstOrigRect.height/2;
    var y2=lastReplRect.top-containerRect.top+lastReplRect.height/2;
    var xR=32,xL=14;var R=Math.min(4,Math.abs(y2-y1)/2);
    var d;
    if(Math.abs(y2-y1)<2){d='M '+xR+' '+y1+' L '+xR+' '+y2;}
    else{d='M '+xR+' '+y1+' L '+(xL+R)+' '+y1+' A '+R+' '+R+' 0 0 1 '+xL+' '+(y1+R)+' L '+xL+' '+(y2-R)+' A '+R+' '+R+' 0 0 0 '+(xL+R)+' '+y2+' L '+xR+' '+y2;}
    var path=document.createElementNS('http://www.w3.org/2000/svg','path');
    path.setAttribute('d',d);path.classList.add('opt-bracket');path.dataset.optGroup=gid;
    svg.appendChild(path);
  }
  container.insertBefore(svg,container.firstChild);
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
    html+='<div class="gvn-item"'+sourceRangeAttrs(w.range)+'><span class="gvn-code">'+esc(w.code)+'</span> '+esc(w.message||'redundant computation')+' <span class="gvn-expr">'+esc(w.expression)+'</span> <span class="gvn-first">[first: '+spanLabel(w.firstRange)+']</span> <span style="color:var(--text-dim); font-size:10px">['+spanLabel(w.range)+']</span></div>';
  }
  pane.innerHTML=html;setupHoverHighlighting(pane);
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
      var severity=w.code.startsWith('T1')||w.code.includes('3001')||w.code.includes('3002')||w.code.includes('3003')||w.code.includes('3004')?'danger':'warn';
      html+='<div class="taint-item taint-'+severity+'"'+sourceRangeAttrs(w.range)+'><span class="taint-code">'+esc(w.code)+'</span> '+esc(w.message)+' ';
      if(w.variable)html+='<span style="color:var(--orange)">'+esc(w.variable)+'</span> ';
      if(w.sinkCommand)html+='<span style="color:var(--red)">\u2192 '+esc(w.sinkCommand)+'</span> ';
      html+='<span style="color:var(--text-dim); font-size:10px">['+spanLabel(w.range)+']</span></div>';
    }
  }
  if(hasTracking){
    html+='<div class="section-header">Taint Tracking</div>';
    for(var func of data.taintTracking){
      html+='<div class="proc-card">';
      html+='<div class="proc-name">'+esc(func.name)+'</div>';
      for(var e of func.entries){html+='<div class="taint-tracking-var">'+esc(e.variable)+'#'+e.version+': <span class="taint-val">'+esc(e.taint)+'</span></div>';}
      html+='</div>';
    }
  }
  pane.innerHTML=html;setupHoverHighlighting(pane);
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
      html+='<div class="type-entry"><span class="type-var">'+esc(e.variable)+'#'+e.version+'</span><span class="type-val type-'+e.kind+'">'+esc(e.type)+'</span></div>';
    }
    html+='</div>';
  }
  pane.innerHTML=html;
}

// Source callouts
function renderCallouts() {
  var pane=$('#pane-callouts');
  if(!data.annotations.length){pane.innerHTML='<div class="empty-state">No annotations</div>';return;}
  var source=getSource();var lines=source.split('\n');
  var lineStarts=[0];for(var i=0;i<source.length;i++){if(source[i]==='\n')lineStarts.push(i+1);}
  function offsetToLine(offset){var lo=0,hi=lineStarts.length-1;while(lo<hi){var mid=(lo+hi+1)>>1;if(lineStarts[mid]<=offset)lo=mid;else hi=mid-1;}return lo;}
  var annotsByLine={};
  for(var ann of data.annotations){var line=offsetToLine(ann.range.startOffset);if(!annotsByLine[line])annotsByLine[line]=[];annotsByLine[line].push(ann);}
  var html='';var gutterWidth=String(lines.length).length;
  for(var i=0;i<lines.length;i++){
    html+='<div class="callout-line"><span class="gutter">'+String(i+1).padStart(gutterWidth)+'</span><span class="code-text">'+esc(lines[i])+'</span></div>';
    if(annotsByLine[i]){for(var ann of annotsByLine[i]){
      var lineStart=lineStarts[i];var startCol=Math.max(0,ann.range.startOffset-lineStart);var endCol=Math.max(startCol,ann.range.endOffset-lineStart);
      var marker=' '.repeat(startCol)+'^'+'-'.repeat(Math.max(0,endCol-startCol));
      var arrow=' '.repeat(startCol)+'+--> '+ann.label;
      html+='<div class="callout-annotation kind-'+ann.kind+'"'+sourceRangeAttrs(ann.range)+'>'+esc(marker)+'\n'+esc(arrow)+'</div>';
    }}
  }
  pane.innerHTML=html;setupHoverHighlighting(pane);
}

// Assembly / WASM helpers
function instrCount(arr) { return arr ? arr.reduce(function(n, f) { return n + f.instrCount; }, 0) : 0; }
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
function renderFuncList(paneId, funcs, emptyMsg) {
  var pane = $('#' + paneId);
  if (!funcs || !funcs.length) { pane.innerHTML = '<div class="empty-state">' + emptyMsg + '</div>'; return; }
  pane.innerHTML = funcListHtml(funcs);
}

function renderAsm() { renderFuncList('pane-asm', data.asm, 'No assembly'); }
function renderWasm() { renderFuncList('pane-wasm', data.wasm, 'No WASM output'); }
function renderAsmOpt() { renderFuncList('pane-asm-opt', data.asmOptimised, 'No optimised assembly (source unchanged by optimiser)'); }
function renderWasmOpt() { renderFuncList('pane-wasm-opt', data.wasmOptimised, 'No optimised WASM output (source unchanged by optimiser)'); }

// SSA variable spans with hover tooltips
function renderVarSpan(name,version,type,lattice,role){
  var cls=role==='def'?'ssa-var ssa-var-def':'ssa-var ssa-var-use';
  var ttData={name:name,version:version};if(type)ttData.type=type;if(lattice)ttData.lattice=lattice;ttData.role=role;
  return '<span class="'+cls+'" data-var=\''+esc(JSON.stringify(ttData))+'\'>'+esc(name)+'#'+version+'</span>';
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
function drawAllCfgEdges(pane,funcs){
  pane.querySelectorAll('.cfg-edges-svg').forEach(function(s){s.remove()});
  for(var func of funcs){
    var container=pane.querySelector('.cfg-edges-container[data-func="'+func.name+'"]');
    if(!container)continue;
    var edges=[];
    for(var block of func.blocks){
      if(!block.successors)continue;
      var term=block.terminator;
      for(var target of block.successors){
        var edgeType='goto';
        if(term&&term.type==='branch')edgeType=target===term.trueTarget?'true':'false';
        edges.push({from:block.name,to:target,edgeType:edgeType});
      }
    }
    if(!edges.length)continue;
    var containerRect=container.getBoundingClientRect();
    var blockEls={};container.querySelectorAll('.cfg-block[data-block]').forEach(function(el){blockEls[el.dataset.block]=el;});
    var svg=document.createElementNS('http://www.w3.org/2000/svg','svg');
    svg.classList.add('cfg-edges-svg');svg.setAttribute('width',containerRect.width);svg.setAttribute('height',containerRect.height);
    var defs=document.createElementNS('http://www.w3.org/2000/svg','defs');
    var fid=func.name.replace(/[^A-Za-z0-9]/g,'_');
    for(var type of ['goto','true','false']){
      var marker=document.createElementNS('http://www.w3.org/2000/svg','marker');
      marker.setAttribute('id','ah-'+type+'-'+fid);marker.setAttribute('viewBox','0 0 8 6');marker.setAttribute('refX','8');marker.setAttribute('refY','3');marker.setAttribute('markerWidth','7');marker.setAttribute('markerHeight','5');marker.setAttribute('orient','auto');
      var poly=document.createElementNS('http://www.w3.org/2000/svg','polygon');poly.setAttribute('points','0 0, 8 3, 0 6');poly.classList.add('cfg-arrowhead','cfg-arrowhead-'+type);
      marker.appendChild(poly);defs.appendChild(marker);
    }
    svg.appendChild(defs);
    var R=4,LANE_W=8,GUTTER_BASE=36;
    for(var i=0;i<edges.length;i++){
      var edge=edges[i];var fromEl=blockEls[edge.from];var toEl=blockEls[edge.to];
      if(!fromEl||!toEl)continue;
      var fromRect=fromEl.getBoundingClientRect();var toRect=toEl.getBoundingClientRect();
      var blockLeft=fromRect.left-containerRect.left;
      var x1=blockLeft,y1=fromRect.bottom-containerRect.top-4;
      var x2=toRect.left-containerRect.left,y2=toRect.top-containerRect.top+8;
      var laneX=GUTTER_BASE-i*LANE_W;var goingDown=y2>y1;
      var d;
      if(Math.abs(y2-y1)<2){d='M '+x1+' '+y1+' L '+x2+' '+y2;}
      else{
        var dy=goingDown?1:-1;
        d='M '+x1+' '+y1+' L '+(laneX+R)+' '+y1+' A '+R+' '+R+' 0 0 '+(goingDown?1:0)+' '+laneX+' '+(y1+dy*R)+' L '+laneX+' '+(y2-dy*R)+' A '+R+' '+R+' 0 0 '+(goingDown?0:1)+' '+(laneX+R)+' '+y2+' L '+x2+' '+y2;
      }
      var path=document.createElementNS('http://www.w3.org/2000/svg','path');
      path.setAttribute('d',d);path.classList.add('cfg-edge','cfg-edge-'+edge.edgeType);
      path.dataset.from=edge.from;path.dataset.to=edge.to;
      path.setAttribute('marker-end','url(#ah-'+edge.edgeType+'-'+fid+')');
      svg.appendChild(path);
    }
    container.insertBefore(svg,container.firstChild);
    container.addEventListener('mouseover',function(e){
      var block=e.target.closest('.cfg-block[data-block]');if(!block)return;
      var bn=block.dataset.block;
      svg.querySelectorAll('.cfg-edge').forEach(function(p){if(p.dataset.from===bn||p.dataset.to===bn)p.classList.add('highlighted');});
    });
    container.addEventListener('mouseout',function(e){
      var block=e.target.closest('.cfg-block[data-block]');if(!block)return;
      svg.querySelectorAll('.cfg-edge.highlighted').forEach(function(p){p.classList.remove('highlighted')});
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
