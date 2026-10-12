set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012cap1/evidence/__tcl_lsp_probe_nh20261012cap1_iapp_apl
E=/var/tmp/tcl_lsp_nh20261012cap1/evidence/__tcl_lsp_probe_nh20261012cap1_iapp_apl
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
run before-0 tmsh list sys application template /Common/__tcl_lsp_probe_nh20261012cap1_iapp_apl
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list sys application service /Common/__tcl_lsp_probe_nh20261012cap1_iapp_apl_service
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012cap1/__tcl_lsp_probe_nh20261012cap1_iapp_apl.conf
if ! grep -q "^7ac99f08125cb049091e9cf746335cfa96ad1553ccead40f0303b1ac60610e9e " "$E/input-hash.stdout"; then exit 4; fi
owned+=('sys application template /Common/__tcl_lsp_probe_nh20261012cap1_iapp_apl')
owned+=('sys application service /Common/__tcl_lsp_probe_nh20261012cap1_iapp_apl_service')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012cap1/__tcl_lsp_probe_nh20261012cap1_iapp_apl.conf
run applied-0 tmsh list sys application template /Common/__tcl_lsp_probe_nh20261012cap1_iapp_apl
run applied-1 tmsh list sys application service /Common/__tcl_lsp_probe_nh20261012cap1_iapp_apl_service
run invoke tmsh modify sys application service /Common/__tcl_lsp_probe_nh20261012cap1_iapp_apl_service execute-action definition
sleep 2
run runtime-logs grep -F 'NHO|nh20261012cap1|' /var/tmp/scriptd.out
echo "configuration_saved=false" > "$E/save-status.txt"
