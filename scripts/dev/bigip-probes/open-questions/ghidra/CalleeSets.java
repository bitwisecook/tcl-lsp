// Dump callee sets (up to a depth) for given handler addresses.
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import java.io.*;
import java.util.*;

public class CalleeSets extends GhidraScript {
    public void run() throws Exception {
        String out = getScriptArgs()[0];
        int depth = Integer.parseInt(getScriptArgs()[1]);
        FunctionManager fm = currentProgram.getFunctionManager();
        AddressSpace sp = currentProgram.getAddressFactory().getDefaultAddressSpace();
        PrintWriter w = new PrintWriter(new FileWriter(out));
        for (int i = 2; i < getScriptArgs().length; i++) {
            String[] nv = getScriptArgs()[i].split("=");
            Function h = fm.getFunctionAt(sp.getAddress(Long.decode(nv[1])));
            Map<Function, Integer> seen = new LinkedHashMap<>();
            ArrayDeque<Function> q = new ArrayDeque<>();
            q.add(h); seen.put(h, 0);
            while (!q.isEmpty()) {
                Function f = q.poll(); int d = seen.get(f);
                if (d >= depth) continue;
                for (Function c : f.getCalledFunctions(monitor)) if (!seen.containsKey(c)) { seen.put(c, d + 1); q.add(c); }
            }
            for (Map.Entry<Function, Integer> e : seen.entrySet())
                w.println(nv[0] + "\t" + e.getKey().getEntryPoint() + "\t" + e.getValue() + "\t" + e.getKey().getName() + "\t" + (e.getKey().isThunk() ? "thunk" : ""));
        }
        w.close();
    }
}
