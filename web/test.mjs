// Runs every example through the WebAssembly build and the browser's WASI
// host, and compares the output with the .out file.   node web/test.mjs
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { argsFor, run, stdinFor } from "./wasi.js";

const root = new URL("..", import.meta.url).pathname;
const module = new WebAssembly.Module(readFileSync(root + "web/cpl.wasm"));
let failed = 0;

function cpl(program, input = "", mode = "run") {
  const text = { 1: "", 2: "" };
  const decoders = { 1: new TextDecoder(), 2: new TextDecoder() };
  const code = run(module, argsFor(program, mode), stdinFor(input), (fd, bytes) => {
    text[fd] += decoders[fd].decode(bytes, { stream: true });
  });
  return { code, out: text[1], err: text[2] };
}

function check(what, ok, detail = "") {
  if (ok) return;
  failed++;
  console.log(`FAIL ${what} ${detail}`);
}

// The .out files record an error as a last line beginning "! ", after a
// line break if the output did not end with one.
function transcript({ out, err }) {
  if (!err) return out;
  const message = err.trimEnd().replace(/^program: /, "");
  return out + (out && !out.endsWith("\n") ? "\n" : "") + `! ${message}\n`;
}

let checked = 0;
for (const file of readdirSync(root + "examples").filter((f) => f.endsWith(".cpl")).sort()) {
  const name = file.slice(0, -4);
  const program = readFileSync(`${root}examples/${file}`, "utf8");
  const inFile = `${root}examples/${name}.in`;
  const input = existsSync(inFile) ? readFileSync(inFile, "utf8") : "";
  const expected = readFileSync(`${root}examples/${name}.out`, "utf8");
  const result = cpl(program, input);
  checked++;
  check(name, transcript(result) === expected && result.code === (result.err ? 1 : 0), `(exit ${result.code})`);
}

// The examples the page puts first must exist.
const shown = JSON.parse(readFileSync(root + "web/examples.json", "utf8"));
check("examples.json", shown.length === checked + 1, `has ${shown.length} programs, examples/ has ${checked} and appendix1/ one`);

// An error must come back as a message and a non-zero exit code.
let r = cpl("let x = 1\nx := 'one'");
check("run-time error", r.code === 1 && r.err.includes("line 2"), JSON.stringify(r));
r = cpl("let x = ");
check("syntax error", r.code === 1 && r.err.includes("line 1"), JSON.stringify(r));

// Recursion is not carried on the engine's stack: this is 50,000 deep.
r = cpl("let rec Sum[integer n] = (n = 0 -> 0, n + Sum[n - 1])\nWrite[Sum[50000]]");
check("deep recursion", r.code === 0 && r.out === "1250025000", JSON.stringify(r));
r = cpl("let rec Forever[integer n] = Forever[n + 1] + 1\nWrite[Forever[1]]");
check("runaway recursion", r.code === 1 && r.err.includes("runaway recursion"), JSON.stringify(r));

// A list of sixty thousand members is made, measured and freed.
r = cpl("let rec Upto[integer n, general l] = (n = 0 -> l, Upto[n - 1, Cons[n, l]])\nWrite[Length[Upto[60000, NIL]]]");
check("long list", r.code === 0 && r.out === "60000", JSON.stringify(r));

// The other things the page can show.
r = cpl("let x = 2\nWrite[x x]", "", "code");
check("machine code", r.code === 0 && r.out.includes("LoadLocal"), JSON.stringify(r));
r = cpl("let x = 2", "", "tree");
check("syntax tree", r.code === 0 && r.out.includes("Simple"), JSON.stringify(r));

r = cpl("let x = 2\nif x > 1 do Write['yes']", "", "printed");
check("publication form", r.code === 0 && r.out.startsWith("l\u0332e\u0332t\u0332 x = 2"), JSON.stringify(r));
// ... which is itself a program.
r = cpl(r.out);
check("publication form runs", r.code === 0 && r.out === "yes", JSON.stringify(r));

// Appendix 1: the preprocessor of 1966, reading a program in the printed alphabet.
const pre = readFileSync(root + "appendix1/preprocessor.cpl", "utf8");
r = cpl(pre, readFileSync(root + "appendix1/sample.pub", "utf8"));
check("appendix 1", r.code === 0 && r.out === readFileSync(root + "appendix1/sample.out", "utf8"), JSON.stringify(r).slice(0, 300));

console.log(`${checked} examples checked through WebAssembly, ${failed} checks failed`);
process.exit(failed ? 1 : 0);
