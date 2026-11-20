// A command suspends if its handler (or a near callee) takes the address of a
// callback that leads to the rule-resume primitive. Output per command:
// name, handler, resume-callbacks taken (with depth), direct-resume flag.
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;
import java.io.*;
import java.util.*;

public class SuspendViaResume extends GhidraScript {
    FunctionManager fm;
    public void run() throws Exception {
        String out = getScriptArgs()[0], table = getScriptArgs()[1];
        fm = currentProgram.getFunctionManager();
        AddressSpace sp = currentProgram.getAddressFactory().getDefaultAddressSpace();
        // function starts from .eh_frame_hdr, to create functions Ghidra missed
        TreeSet<Long> fde = new TreeSet<>();
        BufferedReader fr = new BufferedReader(new FileReader(getScriptArgs()[2]));
        for (String l; (l = fr.readLine()) != null; ) fde.add(Long.decode(l.trim()));
        Set<Function> resume = new HashSet<>();
        for (int i = 3; i < getScriptArgs().length; i++) resume.add(fm.getFunctionAt(sp.getAddress(Long.decode(getScriptArgs()[i]))));
        // create missing functions around call sites of resume, then of their callers
        Set<Function> frontier = new HashSet<>(resume);
        int created = 0;
        for (int level = 0; level < 3; level++) {
            Set<Function> next = new HashSet<>();
            for (Function t : frontier) {
                for (Reference r : getReferencesTo(t.getEntryPoint())) {
                    if (!r.getReferenceType().isCall()) continue;
                    Address from = r.getFromAddress();
                    Function c = fm.getFunctionContaining(from);
                    if (c == null) {
                        Long st = fde.floor(from.getOffset());
                        if (st == null) continue;
                        Address sa = sp.getAddress(st);
                        disassemble(sa);
                        c = createFunction(sa, null);
                        if (c == null) c = fm.getFunctionContaining(from);
                        if (c != null) created++;
                    }
                    if (c != null) next.add(c);
                }
            }
            frontier = next;
        }
        println("created functions: " + created);
        // reverse BFS: functions that reach a resume primitive within 3 calls
        Map<Function, Integer> leadsTo = new HashMap<>();
        ArrayDeque<Function> q = new ArrayDeque<>();
        for (Function r : resume) { leadsTo.put(r, 0); q.add(r); }
        while (!q.isEmpty()) {
            Function f = q.poll(); int d = leadsTo.get(f);
            if (d >= 3) continue;
            for (Function c : f.getCallingFunctions(monitor)) if (!leadsTo.containsKey(c)) { leadsTo.put(c, d + 1); q.add(c); }
        }
        println("functions leading to resume: " + leadsTo.size());
        PrintWriter w = new PrintWriter(new FileWriter(out));
        BufferedReader rd = new BufferedReader(new FileReader(table));
        String ln;
        Listing lst = currentProgram.getListing();
        while ((ln = rd.readLine()) != null) {
            String[] cols = ln.split("\t");
            Function h = fm.getFunctionAt(sp.getAddress(Long.decode(cols[1])));
            if (h == null) continue;
            // callees of the handler up to depth 3
            Map<Function, Integer> seen = new LinkedHashMap<>();
            ArrayDeque<Function> qq = new ArrayDeque<>(); qq.add(h); seen.put(h, 0);
            while (!qq.isEmpty()) {
                Function f = qq.poll(); int d = seen.get(f);
                if (d >= 3) continue;
                for (Function c : f.getCalledFunctions(monitor)) if (!seen.containsKey(c)) { seen.put(c, d + 1); qq.add(c); }
            }
            TreeSet<String> cbs = new TreeSet<>();
            boolean direct = false;
            for (Map.Entry<Function, Integer> e : seen.entrySet()) {
                Function f = e.getKey();
                if (resume.contains(f)) direct = true;
                for (Instruction ins : lst.getInstructions(f.getBody(), true)) {
                    if (ins.getFlowType().isCall()) continue;
                    for (Reference r : ins.getReferencesFrom()) {
                        Function t = fm.getFunctionAt(r.getToAddress());
                        if (t != null && leadsTo.containsKey(t)) cbs.add(t.getName() + "@d" + e.getValue() + "/r" + leadsTo.get(t));
                    }
                }
            }
            w.println(cols[0] + "\t" + cols[1] + "\t" + (direct ? "direct" : "") + "\t" + String.join(" ", cbs));
        }
        w.close();
    }
}
