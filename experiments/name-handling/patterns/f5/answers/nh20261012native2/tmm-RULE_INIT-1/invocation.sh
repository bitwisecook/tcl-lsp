set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012native2/evidence/__tcl_lsp_probe_nh20261012native2
E=/var/tmp/tcl_lsp_nh20261012native2/evidence/__tcl_lsp_probe_nh20261012native2
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native2_provider
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native2_caller
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-2 tmsh list ltm virtual /Common/__tcl_lsp_probe_nh20261012native2_caller_vs
if ! grep -q "was not found" "$E/before-2.stderr" "$E/before-2.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run destination-inventory tmsh list ltm virtual one-line
if grep -q "destination 192.0.2.10:19912 " "$E/destination-inventory.stdout"; then echo "Destination collision" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012native2/__tcl_lsp_probe_nh20261012native2.conf
if ! grep -q "^185f5691cef10697aa183514d6379751db693e15fda6af168c8179f06e126b75 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012native2_provider')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012native2_caller')
owned+=('ltm virtual /Common/__tcl_lsp_probe_nh20261012native2_caller_vs')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012native2/__tcl_lsp_probe_nh20261012native2.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native2_provider
run applied-1 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native2_caller
run applied-2 tmsh list ltm virtual /Common/__tcl_lsp_probe_nh20261012native2_caller_vs
if [ "$(cat "$E/load.exit")" = 0 ] && [ "$(cat "$E/applied-2.exit")" = 0 ]; then touch "$E/traffic-ready"; for n in {1..90}; do [ -e "$E/traffic-done" ] && break; sleep 1; done; fi
sleep 5
run runtime-logs grep -F 'NHO|nh20261012native2|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
