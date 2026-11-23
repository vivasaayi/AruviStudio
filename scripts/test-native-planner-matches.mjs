// Test the browser and real matcher against a disposable native server and synthetic draft.
import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn } from 'node:child_process';
import net from 'node:net';
import { chromium } from '@playwright/test';
const root = await mkdtemp(join(tmpdir(), 'aruvi-planner-match-'));
const token = 'planner-match-fixture-token';
let server, browser;
try {
  const port = await new Promise(resolvePort => { const s = net.createServer(); s.listen(0,'127.0.0.1', () => { const p=s.address().port; s.close(() => resolvePort(p)); }); });
  const base = `http://127.0.0.1:${port}`;
  server = spawn(resolve('src-tauri/target/debug/aruvi-studio-server'), [], {stdio:'ignore',env:{...process.env, ARUVI_APP_DATA_DIR:root, ARUVI_DB_PATH:join(root,'db.sqlite'), ARUVI_WEBHOOK_HOST:'127.0.0.1', ARUVI_WEBHOOK_PORT:String(port), ARUVI_MOBILE_API_TOKEN:token, ARUVI_MCP_API_TOKEN:token}});
  for (let i=0;;i++) { if (server.exitCode !== null || i === 120) throw new Error('Native fixture did not start'); try { if ((await fetch(base+'/health')).ok) break; } catch {} await new Promise(r => setTimeout(r,250)); }
  const actions = [
    {type:'update_capability',target:{productName:'Store',productAreaName:'Warehouse',capabilityName:'Inventory'},fields:{description:'Warehouse only'}},
    {type:'update_capability',target:{productName:'Store',productAreaName:'Sales',capabilityName:'Inventory'},fields:{description:'Sales only'}},
    {type:'update_capability',target:{capabilityName:'Inventory'},fields:{description:'Unscoped action'}}
  ];
  const tree = [{id:'product',label:'Store',node_type:'product',children:[
    {id:'warehouse',label:'Warehouse',node_type:'product_area',children:[{id:'warehouse-inventory',label:'Inventory',node_type:'capability',children:[]}]},
    {id:'sales',label:'Sales',node_type:'product_area',children:[{id:'sales-inventory',label:'Inventory',node_type:'capability',children:[]}]}
  ]}];
  browser = await chromium.launch({executablePath:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const page = await browser.newPage();
  await page.route('**/api/mobile/products', route => route.fulfill({json:[{id:'fixture',name:'Store'}]}));
  await page.route('**/api/mobile/planner/sessions', route => route.fulfill({json:{session_id:'fixture-session'}}));
  await page.route('**/api/mobile/planner/sessions/*/turn', route => route.fulfill({json:{session_id:'fixture-session',assistant_message:'Ready',pending_plan:{actions},draft_tree_nodes:tree}}));
  await page.goto(base+'/remote');
  await page.locator('#tokenInput').fill(token); await page.locator('#saveButton').click();
  await page.locator('#composer').fill('Inspect fixture'); await page.locator('#sendButton').click();
  const inventory = page.locator('#draftTree .tree-node').filter({hasText:'Inventory'});
  await inventory.first().click();
  const details = page.locator('#draftNodeDetails');
  await details.getByText(/Warehouse only/).waitFor();
  let text = await details.textContent(); assert(text.includes('Unscoped action')); assert(!text.includes('Sales only'));
  await inventory.nth(1).click(); await details.getByText(/Sales only/).waitFor();
  text = await details.textContent(); assert(text.includes('Unscoped action')); assert(!text.includes('Warehouse only'));
  const payload = {node:{label:'Inventory',node_type:'capability',ancestors:[{label:'Store',node_type:'product'},{label:'Warehouse',node_type:'product_area'}]},actions};
  const rpc = await fetch(base+'/api/mcp',{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${token}`,Accept:'application/json, text/event-stream'},body:JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/call',params:{name:'aruvi_planner',arguments:{action:'match_planner_actions',arguments:payload}}})});
  assert.deepEqual((await rpc.json()).result.structuredContent.result,[actions[0],actions[2]]);
  console.log('Native browser and MCP filtered conflicting planner scopes and retained unscoped actions.');
} finally { await browser?.close(); if (server && server.exitCode === null && server.signalCode === null) { server.kill(); await new Promise(r => server.once('exit',r)); } await rm(root,{recursive:true,force:true}); }
