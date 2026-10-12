set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012repair2/evidence/__tcl_lsp_probe_nh20261012repair2_icall_1
E=/var/tmp/tcl_lsp_nh20261012repair2/evidence/__tcl_lsp_probe_nh20261012repair2_icall_1
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
run before-0 tmsh list sys icall script /Common/__tcl_lsp_probe_nh20261012repair2_icall_1
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list sys icall handler triggered /Common/__tcl_lsp_probe_nh20261012repair2_icall_1_handler
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012repair2/__tcl_lsp_probe_nh20261012repair2_icall_1.conf
if ! grep -q "^60e6cb609f469b6920f85bb16ed83c6b84c00d99c7e3b9c355e23df4d045e55c " "$E/input-hash.stdout"; then exit 4; fi
owned+=('sys icall script /Common/__tcl_lsp_probe_nh20261012repair2_icall_1')
owned+=('sys icall handler triggered /Common/__tcl_lsp_probe_nh20261012repair2_icall_1_handler')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012repair2/__tcl_lsp_probe_nh20261012repair2_icall_1.conf
run applied-0 tmsh list sys icall script /Common/__tcl_lsp_probe_nh20261012repair2_icall_1
run applied-1 tmsh list sys icall handler triggered /Common/__tcl_lsp_probe_nh20261012repair2_icall_1_handler
run invoke tmsh generate sys icall event name __tcl_lsp_probe_nh20261012repair2_icall_1_event
sleep 5
run runtime-logs grep -F 'NHO|nh20261012repair2|' /var/tmp/scriptd.out
echo "configuration_saved=false" > "$E/save-status.txt"
