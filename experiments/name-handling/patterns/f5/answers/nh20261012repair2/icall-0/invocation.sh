set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012repair2/evidence/__tcl_lsp_probe_nh20261012repair2_icall_0
E=/var/tmp/tcl_lsp_nh20261012repair2/evidence/__tcl_lsp_probe_nh20261012repair2_icall_0
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
run before-0 tmsh list sys icall script /Common/__tcl_lsp_probe_nh20261012repair2_icall_0
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list sys icall handler triggered /Common/__tcl_lsp_probe_nh20261012repair2_icall_0_handler
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012repair2/__tcl_lsp_probe_nh20261012repair2_icall_0.conf
if ! grep -q "^4d45cd4f104a4678c4eace59289a476ddc687f8200dec2c2270caf3c855b68e9 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('sys icall script /Common/__tcl_lsp_probe_nh20261012repair2_icall_0')
owned+=('sys icall handler triggered /Common/__tcl_lsp_probe_nh20261012repair2_icall_0_handler')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012repair2/__tcl_lsp_probe_nh20261012repair2_icall_0.conf
run applied-0 tmsh list sys icall script /Common/__tcl_lsp_probe_nh20261012repair2_icall_0
run applied-1 tmsh list sys icall handler triggered /Common/__tcl_lsp_probe_nh20261012repair2_icall_0_handler
run invoke tmsh generate sys icall event name __tcl_lsp_probe_nh20261012repair2_icall_0_event
sleep 5
run runtime-logs grep -F 'NHO|nh20261012repair2|' /var/tmp/scriptd.out
echo "configuration_saved=false" > "$E/save-status.txt"
