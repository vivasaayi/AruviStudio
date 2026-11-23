// Both desktop and web render this repository-owned workflow topology.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const topology = JSON.parse(readFileSync("src-tauri/resources/workflow-topology.json", "utf8"));
assert(Array.isArray(topology.nodes) && topology.nodes.length > 0, "Workflow nodes are missing");
assert(Array.isArray(topology.links) && Array.isArray(topology.lanes) && topology.lanes.length > 0, "Workflow links or lanes are missing");
const nodes = new Map();
for (const node of topology.nodes) {
  assert(typeof node.id === "string" && node.id && !nodes.has(node.id), `Invalid or duplicate node ID: ${node.id}`);
  assert(typeof node.label === "string" && node.label.trim(), `Node label is missing: ${node.id}`);
  assert(node.webLabel === undefined || (typeof node.webLabel === "string" && node.webLabel.trim()), `Invalid browser label: ${node.id}`);
  assert(node.graphLabel === undefined || (typeof node.graphLabel === "string" && node.graphLabel.trim()), `Invalid diagram label: ${node.id}`);
  assert(Number.isFinite(node.x) && Number.isFinite(node.y), `Invalid node coordinates: ${node.id}`);
  assert(!node.kind || ["stage", "split", "merge"].includes(node.kind), `Invalid node kind: ${node.id}`);
  assert(Array.isArray(node.actualStageIds) && node.actualStageIds.length <= 1 && node.actualStageIds.every((stage) => typeof stage === "string" && stage), `Invalid or unsupported runtime stages: ${node.id}`);
  nodes.set(node.id, node);
}
const edges = new Set(), outgoing = new Map([...nodes.keys()].map((id) => [id, []])), indegree = new Map([...nodes.keys()].map((id) => [id, 0]));
for (const link of topology.links) {
  assert(Array.isArray(link) && link.length === 2, "Invalid workflow link");
  const [from, to] = link, key = JSON.stringify(link);
  assert(nodes.has(from) && nodes.has(to) && from !== to && !edges.has(key), `Invalid or duplicate workflow link: ${key}`);
  edges.add(key); outgoing.get(from).push(to); indegree.set(to, indegree.get(to) + 1);
}
const ready = [...nodes.keys()].filter((id) => indegree.get(id) === 0);
let visited = 0;
while (ready.length) { const id = ready.pop(); visited++; for (const next of outgoing.get(id)) { indegree.set(next, indegree.get(next) - 1); if (indegree.get(next) === 0) ready.push(next); } }
assert.equal(visited, nodes.size, "Workflow topology contains a cycle");
const laneIds = new Set(), assigned = new Set();
for (const lane of topology.lanes) {
  assert(typeof lane.id === "string" && lane.id && !laneIds.has(lane.id), `Invalid or duplicate lane ID: ${lane.id}`);
  assert(typeof lane.label === "string" && lane.label.trim() && [lane.x, lane.y, lane.width, lane.height].every(Number.isFinite) && lane.width > 0 && lane.height > 0, `Invalid lane: ${lane.id}`);
  assert(Array.isArray(lane.nodeIds), `Invalid lane nodes: ${lane.id}`);
  laneIds.add(lane.id);
  for (const id of lane.nodeIds) { assert(nodes.has(id) && !assigned.has(id), `Unknown or duplicate lane node: ${id}`); assigned.add(id); }
}
assert.equal(assigned.size, nodes.size, "Each workflow node must belong to one lane");
const desktop = readFileSync("src/features/work-items/lib/workItemWorkflowDag.ts", "utf8");
const remote = readFileSync("src-tauri/src/services/remote_app.html", "utf8");
const server = readFileSync("src-tauri/src/services/webhook_remote_app.rs", "utf8");
assert(desktop.includes('src-tauri/resources/workflow-topology.json'), "Desktop must import the shared topology");
assert(remote.includes('id="workflowTopology">__ARUVI_WORKFLOW_TOPOLOGY__') && remote.includes('JSON.parse(document.getElementById("workflowTopology").textContent)'), "Browser must use the server-supplied topology");
assert(server.includes('include_str!("../../resources/workflow-topology.json")'), "Server must embed the shared topology");
console.log(`Shared workflow topology guard passed: ${nodes.size} nodes, ${edges.size} connections, ${laneIds.size} lanes`);
