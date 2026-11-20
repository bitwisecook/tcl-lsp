// Commands that consult TMM's thread-local "resuming" flag (fs:-0x1a15570) are
// commands that suspend the rule (return code 5) and are re-executed on resume.
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.mem.*;
import java.io.*;
import java.util.*;

public class ResumeFlag extends GhidraScript {
    public void run() throws Exception {
        String out = getScriptArgs()[0], table = getScriptArgs()[1], fdePath = getScriptArgs()[2];
        int depth = Integer.parseInt(getScriptArgs()[3]);
        Memory mem = currentProgram.getMemory();
        FunctionManager fm = currentProgram.getFunctionManager();
        AddressSpace sp = currentProgram.getAddressFactory().getDefaultAddressSpace();
        TreeSet<Long> fde = new TreeSet<>();
        BufferedReader fr = new BufferedReader(new FileReader(fdePath));
        for (String l; (l = fr.readLine()) != null; ) fde.add(Long.decode(l.trim()));
        byte[] disp = {(byte)0x90, (byte)0xAA, (byte)0x5E, (byte)0xFE};
        MemoryBlock text = mem.getBlock(".text");
        Map<Function, Integer> users = new LinkedHashMap<>();
        Address hit = mem.findBytes(text.getStart(), text.getEnd(), disp, null, true, monitor);
        int hits = 0;
        while (hit != null) {
            hits++;
            Function f = fm.getFunctionContaining(hit);
            if (f == null) {
                Long st = fde.floor(hit.getOffset());
                if (st != null) { disassemble(sp.getAddress(st)); f = createFunction(sp.getAddress(st), null); if (f == null) f = fm.getFunctionContaining(hit); }
            }
            if (f != null) users.put(f, 0);
            hit = mem.findBytes(hit.add(1), text.getEnd(), disp, null, true, monitor);
        }
        println("displacement hits " + hits + " functions " + users.size());
        // reverse BFS to handlers
        Map<Function, Integer> reach = new HashMap<>(users);
        ArrayDeque<Function> q = new ArrayDeque<>(users.keySet());
        while (!q.isEmpty()) {
            Function f = q.poll(); int d = reach.get(f);
            if (d >= depth) continue;
            for (Function c : f.getCallingFunctions(monitor)) if (!reach.containsKey(c)) { reach.put(c, d + 1); q.add(c); }
        }
        PrintWriter w = new PrintWriter(new FileWriter(out));
        for (Function f : users.keySet()) w.println("#user\t" + f.getName() + "\t" + f.getEntryPoint());
        BufferedReader rd = new BufferedReader(new FileReader(table));
        for (String ln; (ln = rd.readLine()) != null; ) {
            String[] cols = ln.split("\t");
            Function h = fm.getFunctionAt(sp.getAddress(Long.decode(cols[1])));
            Integer d = h == null ? null : reach.get(h);
            w.println(cols[0] + "\t" + cols[1] + "\t" + (d == null ? "" : d.toString()));
        }
        w.close();
    }
}
