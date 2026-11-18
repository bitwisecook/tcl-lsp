import json, subprocess, sys, time
log, out, rounds = sys.argv[1], sys.argv[2], [int(r) for r in sys.argv[3:]]
seen = set()
while len(seen) < len(rounds):
    try:
        lines = open(log).read().splitlines()
    except FileNotFoundError:
        time.sleep(5); continue
    start = [json.loads(l) for l in lines if '"start"' in l]
    for l in lines:
        r = json.loads(l)
        if r["event"] == "rss" and r["round"] in rounds and r["round"] not in seen:
            pid = start[0]["pid"]
            res = subprocess.run(["vgdb", "--pid=%d" % pid, "detailed_snapshot", f"{out}-{r['round']}.ms"], capture_output=True, text=True, timeout=1800)
            print(r["round"], res.returncode, res.stdout[-200:], res.stderr[-200:], flush=True)
            seen.add(r["round"])
    time.sleep(5)
