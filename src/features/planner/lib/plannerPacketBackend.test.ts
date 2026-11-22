// This test runs in Vitest's Node environment.
// @ts-expect-error Node types are intentionally absent from the browser tsconfig.
import { readFileSync } from "node:fs";
import { buildSync } from "esbuild";
import { expect, it } from "vitest";
import { renderJson } from "./plannerPacketBackend";

it("keeps the embedded Planner renderer aligned with its source", () => {
  const result = buildSync({ entryPoints: ["src/features/planner/lib/plannerPacketBackend.ts"], bundle: true,
    platform: "neutral", format: "iife", globalName: "AruviPlannerPacket", target: "es2020",
    outfile: "src-tauri/resources/planner_packet_renderer.js", write: false });
  expect(result.outputFiles[0].text).toBe(readFileSync("src-tauri/resources/planner_packet_renderer.js", "utf8"));
});

it("renders an explicit no-session snapshot without mutating it", () => {
  const snapshot = { title:"<Inventory>", generatedAt:"fixture", activeProductName:null, currentProducts:[], currentProductTrees:[],
    currentWorkItems:[], currentWorkItemsHasMore:false, draftTreeNodes:[], plan:null, validation:{score:0,issues:[]}, selectedNode:null, latestAssistantText:null };
  const encoded = JSON.stringify(snapshot);
  const packet = JSON.parse(renderJson(encoded));
  expect(packet.filename).toBe("inventory-design-review-packet.html");
  expect(packet.html).toContain("&lt;Inventory&gt;");
  expect(packet.html).toContain("caller-supplied snapshot");
  expect(packet.html).not.toContain("<Inventory>");
  expect(JSON.stringify(snapshot)).toBe(encoded);
});

it("keeps supplied context, draft, actions and validation in the packet", () => {
  const input = {title:"Inventory",generatedAt:"fixture",activeProductName:"Shop",currentProducts:[{}],
    currentProductTrees:[{roots:[{}]}],currentWorkItems:[{}],currentWorkItemsHasMore:true,
    draftTreeNodes:[{id:"area",label:"Stock <&>",node_type:"product_area",children:[]}],
    plan:{actions:[{type:"create_work_item",title:"Count stock"}]},
    validation:{score:72,issues:[{tone:"warn",title:"Review risk",detail:"Missing owner"}]},
    selectedNode:null,latestAssistantText:"Review the plan"};
  const packet = JSON.parse(renderJson(JSON.stringify(input)));
  for (const text of ["Shop", "Stock &lt;&amp;&gt;", "Count stock", "Review risk", "Missing owner", "first 1 story/task"]) {
    expect(packet.html).toContain(text);
  }
});

it("renders an offline, escaped overview and complete hierarchy beyond the visual limit", () => {
  const nodes = Array.from({ length: 101 }, (_, i) => ({
    id: `node-${i}`, label: i === 100 ? 'Last <script>alert("x")</script>' : `Node ${i}`,
    node_type: "product_area", children: i === 0 ? [{ id: "child", label: "Child & sibling", node_type: "product_area", children: [] }] : [],
  }));
  const snapshot = { title:"Offline", generatedAt:"fixture", activeProductName:null, currentProducts:[], currentProductTrees:[],
    currentWorkItems:[], currentWorkItemsHasMore:false, draftTreeNodes:nodes, plan:null, validation:{score:0,issues:[]}, selectedNode:null };
  const { html } = JSON.parse(renderJson(JSON.stringify(snapshot)));
  expect(html).toContain('aria-label="Staged design hierarchy"');
  expect(html).toContain('aria-label="Complete staged design hierarchy"');
  expect(html).toContain('Diagram shows the first 100 nodes');
  expect(html).toContain('Child &amp; sibling');
  expect(html).toContain('Last &lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;');
  expect(html).not.toContain('<script');
  expect(html).not.toContain('cdn.jsdelivr.net');
});


it("escapes a caller-supplied readiness score", () => {
  const snapshot = { title:"Review", generatedAt:"fixture", activeProductName:null, currentProducts:[], currentProductTrees:[],
    currentWorkItems:[], currentWorkItemsHasMore:false, draftTreeNodes:[], plan:null,
    validation:{score:'</div><img src=x onerror=alert(1)>',issues:[]}, selectedNode:null };
  const { html } = JSON.parse(renderJson(JSON.stringify(snapshot)));
  expect(html).toContain('&lt;/div&gt;&lt;img src=x onerror=alert(1)&gt;');
  expect(html).not.toContain('<img src=x onerror=alert(1)>');
});
