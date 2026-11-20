// For each TMM Tcl command (from the set_tclcmd registration table), find the
// handler, the strings it references directly, and which "anchor" functions
// (identified by the strings they reference) it can reach through calls.
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.mem.*;
import ghidra.program.model.symbol.*;
import java.io.*;
import java.util.*;

public class TclCmdEffects extends GhidraScript {
    // tag -> substrings identifying anchor functions
    static final String[][] ANCHORS = {
        {"suspend", "cannot suspend: No connflow/tcl context"},
        {"suspend", "Operation need to suspend"},
        {"suspend", "Suspending command returned to incorrect context"},
        {"parked", "while the interpreter is parked"},
    };
    static final int DEPTH = 6;

    String strAt(Address a) {
        try {
            StringBuilder sb = new StringBuilder();
            Memory m = currentProgram.getMemory();
            for (int i = 0; i < 200; i++) {
                byte b = m.getByte(a.add(i));
                if (b == 0) break;
                if (b < 32 || b > 126) return null;
                sb.append((char) b);
            }
            return sb.length() >= 3 ? sb.toString() : null;
        } catch (Exception e) { return null; }
    }

    Set<String> directStrings(Function f) {
        Set<String> out = new LinkedHashSet<>();
        Listing l = currentProgram.getListing();
        for (Instruction ins : l.getInstructions(f.getBody(), true)) {
            for (Reference r : ins.getReferencesFrom()) {
                Address to = r.getToAddress();
                if (!to.isMemoryAddress()) continue;
                String s = strAt(to);
                if (s != null) out.add(s);
            }
        }
        return out;
    }

    public void run() throws Exception {
        String outPath = getScriptArgs()[0];
        Memory mem = currentProgram.getMemory();
        MemoryBlock blk = mem.getBlock("set_tclcmd");
        FunctionManager fm = currentProgram.getFunctionManager();
        AddressSpace sp = currentProgram.getAddressFactory().getDefaultAddressSpace();

        // anchors: functions referencing anchor strings
        Map<Function, String> anchor = new HashMap<>();
        for (MemoryBlock b : mem.getBlocks()) {
            if (!b.getName().equals(".rodata")) continue;
            for (String[] a : ANCHORS) {
                Address hit = mem.findBytes(b.getStart(), b.getEnd(), a[1].getBytes(), null, true, monitor);
                while (hit != null) {
                    // walk back to string start
                    Address st = hit;
                    while (mem.getByte(st.subtract(1)) != 0) st = st.subtract(1);
                    for (Reference r : getReferencesTo(st)) {
                        Function f = fm.getFunctionContaining(r.getFromAddress());
                        if (f != null) anchor.put(f, a[0]);
                    }
                    hit = mem.findBytes(hit.add(1), b.getEnd(), a[1].getBytes(), null, true, monitor);
                }
            }
        }
        println("anchor functions: " + anchor.size());

        println("block " + blk + " start " + blk.getStart() + " end " + blk.getEnd());
        int nNull = 0, nFn = 0, nRows = 0;
        PrintWriter w = new PrintWriter(new FileWriter(outPath));
        // the registration table comes from the raw file (name, handler, clientdata):
        // Ghidra does not load set_tclcmd's contents
        BufferedReader rd = new BufferedReader(new FileReader(getScriptArgs()[1]));
        String ln;
        while ((ln = rd.readLine()) != null) {
            String[] cols = ln.split("\t");
            String name = cols[0];
            long pFn = Long.decode(cols[1]);
            long cd = Long.decode(cols[2]);
            Address fa = sp.getAddress(pFn);
            Function h = fm.getFunctionAt(fa);
            if (h == null) { createFunction(fa, null); h = fm.getFunctionAt(fa); }
            if (name == null || name.equals("None") || h == null) { nNull++; continue; }
            nRows++;
            // BFS
            Map<String, Integer> reach = new TreeMap<>();
            Map<Function, Integer> seen = new HashMap<>();
            ArrayDeque<Function> q = new ArrayDeque<>();
            q.add(h); seen.put(h, 0);
            while (!q.isEmpty()) {
                Function f = q.poll();
                int d = seen.get(f);
                if (anchor.containsKey(f)) reach.merge(anchor.get(f), d, Math::min);
                if (d >= DEPTH) continue;
                for (Function c : f.getCalledFunctions(monitor)) {
                    if (!seen.containsKey(c)) { seen.put(c, d + 1); q.add(c); }
                }
            }
            Set<String> ds = directStrings(h);
            List<String> dl = new ArrayList<>(ds);
            if (dl.size() > 25) dl = dl.subList(0, 25);
            w.println(name + "\t" + h.getEntryPoint() + "\t" + cd + "\t" + reach + "\t" + seen.size() + "\t" + String.join(" || ", dl).replace("\t", " ").replace("\n", " "));
        }
        w.close();
        println("rows " + nRows + " skipped " + nNull);
    }
}
