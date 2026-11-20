#!/bin/bash
# Appliance-side probe: realm scope of `tmsh::modify cli version active`.
#
# Distinguishes script, tmsh process, session (a process and its children)
# and system scope by reading the active version from each place after a
# change. Creates only tcllsp_oq_* cli scripts, restores the version it
# found, deletes everything it created, never saves config.
set -u
P=tcllsp_oq
ACT() { tmsh show cli version 2>&1 | awk '/active/{print $2}'; }

echo "SYS~~$(tmsh show sys version | awk '/ Version | Build | Edition /{printf "%s=%s ", $1, $2}')"
ORIG=$(ACT)
echo "VER~~initial~~$ORIG"

cat > /var/tmp/$P-scripts.conf <<'EOF'
cli script tcllsp_oq_ver_a {
proc active {} {
    regexp {active (\S+)} [tmsh::show cli version] -> v
    return $v
}
proc script::run {} {
    puts "A-before [active]"
    tmsh::modify cli version active 14.0.0
    puts "A-after-modify [active]"
    tmsh::run cli script tcllsp_oq_ver_b
    puts "A-after-nested-script [active]"
    puts "A-child-process [string trim [exec tmsh show cli version | awk {/active/{print $2}}]]"
}
}
cli script tcllsp_oq_ver_b {
proc script::run {} {
    regexp {active (\S+)} [tmsh::show cli version] -> v
    puts "B-nested-script $v"
}
}
EOF
for s in ver_a ver_b; do
  tmsh list cli script ${P}_$s >/dev/null 2>&1 && { echo "ABORT~~${P}_$s exists"; exit 1; }
done
tmsh load sys config merge file /var/tmp/$P-scripts.conf >/dev/null 2>&1 || echo "LOAD~~failed"

echo "== 1. one cli script: modify, nested script, child process"
tmsh run cli script ${P}_ver_a 2>&1 | sed 's/^/SCRIPT~~/'
echo "VER~~fresh-process-after-script~~$(ACT)"

echo "== 2. one tmsh process, several commands"
tmsh -c "modify cli version active 17.5.0; show cli version; run cli script ${P}_ver_b; run util bash -c 'tmsh show cli version'" 2>&1 \
  | awk '/active|B-nested/' | sed 's/^/SESSION~~/'
echo "VER~~fresh-process-after-session~~$(ACT)"

echo "== 3. standalone modify, then a fresh process"
tmsh modify cli version active 14.0.0 2>&1 | sed 's/^/MODIFY~~/'
echo "VER~~fresh-process-after-standalone-modify~~$(ACT)"
grep -l 'cli version' /config/bigip_base.conf /config/bigip.conf /config/bigip_user.conf /config/bigip_script.conf 2>/dev/null | sed 's/^/STORED~~/'

tmsh modify cli version active "$ORIG" >/dev/null 2>&1
for s in ver_a ver_b; do tmsh delete cli script ${P}_$s >/dev/null 2>&1
  tmsh list cli script ${P}_$s >/dev/null 2>&1 && echo "LEFTOVER~~${P}_$s"; done
echo "VER~~final~~$(ACT)"
rm -f /var/tmp/$P-scripts.conf
