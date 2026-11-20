// Tag command handlers that can return Tcl code 5 (F5's suspend code): a
// function "returns 5" if it loads 5 into EAX within a few instructions of a
// RET. Handlers reaching such a function within DEPTH calls are candidates.
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import java.util.regex.*;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.scalar.Scalar;
import java.io.*;
import java.util.*;

public class ReturnFive extends GhidraScript {
    Map<Function, Boolean> memo = new HashMap<>();
    DecompInterface di;
    // "return 5;" or "x = 5; ... return x;" in the decompiled C
    static final Pattern RET5 = Pattern.compile("return 5;");
    static final Pattern ASSIGN5 = Pattern.compile("(\\w+) = 5;");
    boolean returnsFive(Function f) {
        Boolean b = memo.get(f);
        if (b != null) return b;
        boolean r = false;
        DecompileResults res = di.decompileFunction(f, 30, monitor);
        if (res.decompileCompleted()) {
            String c = res.getDecompiledFunction().getC();
            if (RET5.matcher(c).find()) r = true;
            else {
                Matcher m = ASSIGN5.matcher(c);
                while (m.find()) { if (c.contains("return " + m.group(1) + ";")) { r = true; break; } }
            }
        }
        memo.put(f, r);
        return r;
    }
    public void run() throws Exception {
        String out = getScriptArgs()[0], table = getScriptArgs()[1];
        int depth = Integer.parseInt(getScriptArgs()[2]);
        di = new DecompInterface(); di.openProgram(currentProgram);
        FunctionManager fm = currentProgram.getFunctionManager();
        AddressSpace sp = currentProgram.getAddressFactory().getDefaultAddressSpace();
        PrintWriter w = new PrintWriter(new FileWriter(out));
        BufferedReader rd = new BufferedReader(new FileReader(table));
        for (String ln; (ln = rd.readLine()) != null; ) {
            String[] cols = ln.split("\t");
            Function h = fm.getFunctionAt(sp.getAddress(Long.decode(cols[1])));
            if (h == null) { w.println(cols[0] + "\t" + cols[1] + "\t?"); continue; }
            Map<Function, Integer> seen = new LinkedHashMap<>();
            ArrayDeque<Function> q = new ArrayDeque<>(); q.add(h); seen.put(h, 0);
            int best = -1; String via = "";
            while (!q.isEmpty()) {
                Function f = q.poll(); int d = seen.get(f);
                if (returnsFive(f)) { if (best < 0 || d < best) { best = d; via = f.getName(); } }
                if (d >= depth) continue;
                for (Function c : f.getCalledFunctions(monitor)) if (!seen.containsKey(c)) { seen.put(c, d + 1); q.add(c); }
            }
            w.println(cols[0] + "\t" + cols[1] + "\t" + (best < 0 ? "" : best + "\t" + via));
        }
        w.close();
        println("memo " + memo.size());
    }
}
