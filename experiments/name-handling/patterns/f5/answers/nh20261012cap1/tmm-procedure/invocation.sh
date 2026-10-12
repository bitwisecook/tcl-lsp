set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012cap1/evidence/__tcl_lsp_probe_nh20261012cap1_tmm_procedure
E=/var/tmp/tcl_lsp_nh20261012cap1/evidence/__tcl_lsp_probe_nh20261012cap1_tmm_procedure
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
run before-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012cap1_tmm_procedure
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012cap1/__tcl_lsp_probe_nh20261012cap1_tmm_procedure.conf
if ! grep -q "^931a8b149791944c4ef0bcc418fc2c119ff5fe3c770f0cb322e1955a80a9285c " "$E/input-hash.stdout"; then exit 4; fi
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012cap1_tmm_procedure')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012cap1/__tcl_lsp_probe_nh20261012cap1_tmm_procedure.conf
run applied-0 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012cap1_tmm_procedure
sleep 2
run runtime-logs grep -F 'NHO|nh20261012cap1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
