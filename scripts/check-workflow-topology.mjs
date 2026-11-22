// Guard the independently rendered browser diagram until both clients share topology data.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const desktopSource = readFileSync("src/features/work-items/lib/workItemWorkflowDag.ts", "utf8");
const compiled = ts.transpileModule(desktopSource, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2021 } }).outputText;
const desktopContext = { exports: {} };
vm.runInNewContext(compiled, desktopContext, { timeout: 1000 });
const desktop = desktopContext.exports;
const html = readFileSync("src-tauri/src/services/remote_app.html", "utf8");
const start = html.indexOf("            const graphNodes = [");
const end = html.indexOf("            const graphById =", start);
assert(start >= 0 && end > start, "Browser workflow diagram definitions are missing");
const browser = vm.runInNewContext(html.slice(start, end) + ";({graphNodes,graphLinks})", {}, { timeout: 1000 });
const aliases = { requirements: "requirement_analysis", plan_split: "planning_split", architecture: "architecture_plan", unit_plan: "unit_test_plan", integration_plan: "integration_test_plan", ui_plan: "ui_test_plan", plan_merge: "planning_merge", verify_split: "verification_split", unit: "unit_test_generation", integration: "integration_test_generation", ui: "ui_test_planning", qa: "qa_validation", security: "security_review", performance: "performance_review", test_review: "verification_merge", push_prep: "push_preparation" };
const nodeId = (id) => aliases[id] || id;
const links = (pairs, convert) => Array.from(pairs, ([from, to]) => `${convert(from)}:${convert(to)}`).sort();
assert.equal(browser.graphNodes.length, desktop.WORKFLOW_DAG_NODES.length, "Workflow node counts differ");
assert.deepEqual(links(browser.graphLinks, nodeId), links(desktop.WORKFLOW_DAG_LINKS, (id) => id), "Workflow connections differ");
for (const node of browser.graphNodes) {
  const source = desktop.WORKFLOW_DAG_NODES.find((item) => item.id === nodeId(node[0]));
  assert(source, `Unknown browser workflow node: ${node[0]}`);
  assert.equal(node[3], source.x, `Node x differs: ${node[0]}`);
  assert.equal(node[4], source.y, `Node y differs: ${node[0]}`);
  assert.equal(source.actualStageIds.length, node[2] ? 1 : 0, `Runtime stage count differs: ${node[0]}`);
  assert.equal(node[2], source.actualStageIds[0] || "", `Runtime stage differs: ${node[0]}`);
  assert.equal(Boolean(node[5]), Boolean(source.kind && source.kind !== "stage"), `Node shape differs: ${node[0]}`);
}
assert.equal(new Set(Array.from(browser.graphNodes, (node) => nodeId(node[0]))).size, desktop.WORKFLOW_DAG_NODES.length, "Browser workflow node IDs are not unique");
console.log(`Workflow topology guard passed: ${browser.graphNodes.length} nodes and ${browser.graphLinks.length} connections`);
