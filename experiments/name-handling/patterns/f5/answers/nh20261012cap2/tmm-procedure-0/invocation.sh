set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012cap2/evidence/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_0
E=/var/tmp/tcl_lsp_nh20261012cap2/evidence/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_0
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_0
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012cap2/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_0.conf
if ! grep -q "^872d6cf84da5c857fb6c696700e94bcdd4090099de7a501ab7a0436ef7f5885c " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_0')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012cap2/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_0.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_0
sleep 2
run runtime-logs grep -F 'NHO|nh20261012cap2|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
