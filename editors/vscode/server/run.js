// Runs a CPL program:  node run.js program.cpl
// The system is given the text of the program as an argument, so it needs
// no access to files, and the program's own input comes from standard input.
"use strict";

const fs = require("fs");
const path = require("path");
const { run } = require("./wasi.js");

const file = process.argv[2];
if (!file) {
  console.error("usage: run.js program.cpl");
  process.exit(2);
}
let program;
try {
  program = fs.readFileSync(file, "utf8");
} catch (error) {
  console.error(`cpl: cannot read ${file}: ${error.message}`);
  process.exit(2);
}
process.exitCode = run(path.join(__dirname, "cpl.wasm"), { args: ["-e", program] });
