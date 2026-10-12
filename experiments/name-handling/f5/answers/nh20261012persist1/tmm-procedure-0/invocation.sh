set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012persist1/evidence/__tcl_lsp_probe_nh20261012persist1
E=/var/tmp/tcl_lsp_nh20261012persist1/evidence/__tcl_lsp_probe_nh20261012persist1
owned=()
run() { label=$1; shift; "$@" > "$E/$label.stdout" 2> "$E/$label.stderr"; echo $? > "$E/$label.exit"; }
cleanup() {
  for (( i=${#owned[@]}-1; i>=0; i-- )); do
    read -r -a words <<< "${owned[$i]}"
    run "delete-$i" tmsh delete "${words[@]}"
    run "after-$i" tmsh list "${words[@]}"
  done
}
trap cleanup EXIT
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012persist1_provider
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012persist1
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-2 tmsh list ltm virtual /Common/__tcl_lsp_probe_nh20261012persist1_vs
if ! grep -q "was not found" "$E/before-2.stderr" "$E/before-2.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run destination-inventory tmsh list ltm virtual one-line
if grep -q "destination 192.168.9.24:19912 " "$E/destination-inventory.stdout"; then echo "Destination collision" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012persist1/__tcl_lsp_probe_nh20261012persist1.conf
if ! grep -q "^fdf85f2150e124f05fd4b828145c7b7d864005b4e97eabce319ce538ec4203b7 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012persist1_provider')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012persist1')
owned+=('ltm virtual /Common/__tcl_lsp_probe_nh20261012persist1_vs')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012persist1/__tcl_lsp_probe_nh20261012persist1.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012persist1_provider
run applied-1 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012persist1
run applied-2 tmsh list ltm virtual /Common/__tcl_lsp_probe_nh20261012persist1_vs
if [ "$(cat "$E/load.exit")" = 0 ] && [ "$(cat "$E/applied-2.exit")" = 0 ]; then touch "$E/traffic-ready"; for n in {1..90}; do [ -e "$E/traffic-done" ] && break; sleep 1; done; fi
sleep 5
run runtime-logs grep -F 'NHO|nh20261012persist1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
