// Runs inside VS Code. Exercises the extension the way a person would: open
// a program, hover, jump, rename, see problems, and run it.
"use strict";

const assert = require("node:assert/strict");
const path = require("node:path");
const vscode = require("vscode");

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function until(what, probe, seconds = 20) {
  for (let i = 0; i < seconds * 10; i++) {
    const value = await probe();
    if (value) return value;
    await sleep(100);
  }
  throw new Error(`timed out waiting for ${what}`);
}

const hoverText = (hovers) =>
  hovers.flatMap((h) => h.contents).map((c) => (typeof c === "string" ? c : c.value)).join("\n");

exports.run = async function run() {
  const folder = vscode.workspace.workspaceFolders[0].uri.fsPath;
  const document = await vscode.workspace.openTextDocument(path.join(folder, "manual-hcf.cpl"));
  await vscode.window.showTextDocument(document);
  assert.equal(document.languageId, "cpl", "the .cpl extension selects the language");

  const call = new vscode.Position(7, 28); // the HCF of "HCF[1071, 462]", outside the string
  assert.equal(document.lineAt(7).text, "Write['HCF[1071, 462] = ', HCF[1071, 462], '*n']");

  // The server is ready when it answers a hover.
  const hovers = await until("a hover from the language server", async () => {
    const found = await vscode.commands.executeCommand("vscode.executeHoverProvider", document.uri, call);
    return found.length ? found : undefined;
  });
  assert.match(hoverText(hovers), /Function `HCF`/);
  assert.match(hoverText(hovers), /let rec HCF\[integer n, m\] =/);
  const word = await vscode.commands.executeCommand("vscode.executeHoverProvider", document.uri, new vscode.Position(2, 5));
  assert.match(hoverText(word), /`rec`.*recursive definition/);
  const basic = await vscode.commands.executeCommand("vscode.executeHoverProvider", document.uri, new vscode.Position(5, 16));
  assert.match(hoverText(basic), /`Rem\[x, y\]`.*basic function/);
  console.log("hover ok");

  const definitions = await vscode.commands.executeCommand("vscode.executeDefinitionProvider", document.uri, call);
  assert.equal(definitions.length, 1);
  assert.equal((definitions[0].range || definitions[0].targetRange).start.line, 2);
  console.log("definition ok");

  const references = await vscode.commands.executeCommand("vscode.executeReferenceProvider", document.uri, call);
  assert.equal(references.length, 5);

  const edit = await vscode.commands.executeCommand("vscode.executeDocumentRenameProvider", document.uri, call, "Gcd");
  assert.equal(edit.entries()[0][1].length, 5, "renaming a function changes its definition and every call");
  await assert.rejects(
    vscode.commands.executeCommand("vscode.executeDocumentRenameProvider", document.uri, call, "Rem"),
    /Rem is the name of a basic function/
  );
  console.log("references and rename ok");

  const symbols = await vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", document.uri);
  assert.deepEqual(symbols.map((s) => s.name), ["HCF"]);

  const legend = await vscode.commands.executeCommand("vscode.provideDocumentSemanticTokensLegend", document.uri);
  assert.ok(legend.tokenTypes.includes("parameter"));
  const tokens = await vscode.commands.executeCommand("vscode.provideDocumentSemanticTokens", document.uri);
  assert.ok(tokens.data.length >= 5 * 40);
  console.log("symbols and semantic tokens ok");

  const completions = await vscode.commands.executeCommand("vscode.executeCompletionItemProvider", document.uri, new vscode.Position(7, 0));
  const labels = completions.items.map((i) => (typeof i.label === "string" ? i.label : i.label.label));
  for (const wanted of ["HCF", "Sqrt", "let", "value of"]) assert.ok(labels.includes(wanted), wanted);

  const help = await vscode.commands.executeCommand("vscode.executeSignatureHelpProvider", document.uri, new vscode.Position(7, 36), ",");
  assert.equal(help.signatures[0].label, "HCF[n, m]");
  assert.equal(help.activeParameter, 1);
  console.log("completion and signature help ok");

  assert.deepEqual(vscode.languages.getDiagnostics(document.uri), [], "the program has no problems");

  // A broken program shows its problem.
  const broken = await vscode.workspace.openTextDocument({ language: "cpl", content: "let count = 0\n" });
  await vscode.window.showTextDocument(broken);
  const problems = await until("diagnostics", () => {
    const found = vscode.languages.getDiagnostics(broken.uri);
    return found.length ? found : undefined;
  });
  assert.deepEqual(problems.map((p) => p.code), ["syntax"]);
  assert.equal(problems[0].severity, vscode.DiagnosticSeverity.Error);
  assert.equal(problems[0].source, "cpl");
  assert.equal(problems[0].range.start.character, 5);
  console.log("diagnostics ok");

  // Run Program: a task that ends with exit code 0.
  await vscode.window.showTextDocument(document);
  const finished = new Promise((resolve) => {
    const listener = vscode.tasks.onDidEndTaskProcess((event) => {
      if (event.execution.task.definition.type === "cpl") {
        listener.dispose();
        resolve(event.exitCode);
      }
    });
  });
  await vscode.commands.executeCommand("cpl.run");
  assert.equal(await Promise.race([finished, sleep(20000).then(() => "timed out")]), 0);
  console.log("run ok");
};
