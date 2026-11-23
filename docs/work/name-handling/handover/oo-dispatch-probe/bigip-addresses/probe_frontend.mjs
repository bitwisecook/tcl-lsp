// Q8 probe: verbatim copies of rust/bigip-report-gen/frontend/src/pages/topology.ts helpers at b3f2f32c (lines 1082-1120, 1607-1640).
// Run: node probe_frontend.mjs > matrix-frontend.tsv
import { readFileSync } from "node:fs";
  function ipVer(ip) {
    if (!ip) return null;
    if (ip.indexOf(":") >= 0) return 6;
    if (/^\d+\.\d+\.\d+\.\d+$/.test(ip)) return 4;
    return null;
  }
  function v6ToBig(ip) {
    ip = ip.split("%")[0];
    var halves = ip.split("::");
    if (halves.length > 2) return null;
    var head = halves[0] ? halves[0].split(":") : [];
    var groups;
    if (halves.length === 2) {
      var tail = halves[1] ? halves[1].split(":") : [];
      var mid = 8 - head.length - tail.length;
      if (mid < 0) return null;
      groups = head.concat(new Array(mid).fill("0"), tail);
    } else {
      groups = head;
      if (groups.length !== 8) return null;
    }
    var big = 0n;
    for (var i = 0; i < 8; i++) {
      var g = groups[i] || "0";
      if (!/^[0-9a-fA-F]{1,4}$/.test(g)) return null;
      big = (big << 16n) + BigInt(parseInt(g, 16));
    }
    return big;
  }
  function ipToBig(ip) {
    var v = ipVer(ip);
    if (v === 4) {
      var p = ip.split("%")[0].split(".").map(Number);
      if (p.some(function (x) { return isNaN(x) || x < 0 || x > 255; })) return null;
      return (BigInt(p[0]) << 24n) + (BigInt(p[1]) << 16n) + (BigInt(p[2]) << 8n) + BigInt(p[3]);
    }
    if (v === 6) return v6ToBig(ip);
    return null;
  }
    function toNet(s) {
      var m = /^([0-9a-fA-F:.]+)(?:\/(\d+))?$/.exec(s);
      if (!m) return null;
      var ver = ipVer(m[1]); if (!ver) return null;
      var big = ipToBig(m[1]); if (big == null) return null;
      var full = ver === 6 ? 128 : 32;
      var pfx = m[2] !== undefined ? parseInt(m[2], 10) : full;
      if (pfx > full) return null;
      return { ver: ver, big: big, prefix: pfx, full: full };
    }
    // Does `outer` (the less-specific network) contain `inner`?
    function contains(outer, inner) {
      if (outer.ver !== inner.ver || inner.prefix < outer.prefix) return false;
      var mask = ((1n << BigInt(outer.prefix)) - 1n) << BigInt(outer.full - outer.prefix);
      return (outer.big & mask) === (inner.big & mask);
    }
    function ipOverlap(a, b) { return contains(a, b) || contains(b, a); }
    var IP_TOKEN = /[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+(?:\/[0-9]+)?|[0-9a-fA-F:]{2,}:[0-9a-fA-F:]*(?:\/[0-9]+)?/g;

    // Parse a search query as an address, honouring BIG-IP `%route-domain` and
    // an optional `:port` — e.g. `10.0.0.1%2:8443`, `192.168.1.0/24`, `::1`.
    function parseIpQuery(raw) {
      var s = raw.trim();
      if (!s) return null;
      var rd = null, port = null;
      // trailing :port only for IPv4 (or bracketed IPv6) to avoid eating v6 colons
      var pm = /^(.*?):(\d+)$/.exec(s);
      if (pm && pm[1].indexOf(".") >= 0) { s = pm[1]; port = pm[2]; }
      var rm = /^([^%]*)%(\d+)(.*)$/.exec(s);
      if (rm) { rd = parseInt(rm[2], 10); s = rm[1] + rm[3]; }
      var net = toNet(s);
      if (!net) return null;
      return { net: net, rd: rd, port: port, base: s.split("/")[0] };
    }
const dir = new URL(".", import.meta.url).pathname;
const lines = readFileSync(dir + "inputs.tsv", "utf8").split("\n").slice(1).filter(Boolean);
const show = (v) => JSON.stringify(v, (k, x) => typeof x === "bigint" ? x.toString(16) : x);
console.log("id\tinput\tfunction\tverdict\tparts");
for (const l of lines) {
  const [id, , s] = l.split("\t");
  const q = parseIpQuery(s);
  console.log(`${id}\t${s}\tfrontend topology.ts parseIpQuery\t${q ? "ok" : "err"}\t${q ? show({ ver: q.net.ver, big: q.net.big, prefix: q.net.prefix, rd: q.rd, port: q.port, base: q.base }) : "null"}`);
  const t = s.match(IP_TOKEN);
  console.log(`${id}\t${s}\tfrontend topology.ts IP_TOKEN\tok\t${show(t)}`);
  console.log(`${id}\t${s}\tfrontend topology.ts ipVer/ipToBig\tok\t${show({ ver: ipVer(s), big: ipToBig(s) })}`);
}
