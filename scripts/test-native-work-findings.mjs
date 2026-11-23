// Verify work-item findings in a disposable server and browser.
import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn, execFileSync } from 'node:child_process';
import net from 'node:net';
import { chromium } from '@playwright/test';

const root = await mkdtemp(join(tmpdir(), 'aruvi-work-findings-'));
let server, browser;
try {
  const port = await new Promise(done => { const socket = net.createServer(); socket.listen(0, '127.0.0.1', () => { const value = socket.address().port; socket.close(() => done(value)); }); });
  const base = `http://127.0.0.1:${port}`, token = 'work-findings-fixture', db = join(root, 'db.sqlite');
  server = spawn(resolve('src-tauri/target/debug/aruvi-studio-server'), [], { stdio:'ignore', env:{ ...process.env, ARUVI_APP_DATA_DIR:root, ARUVI_DB_PATH:db, ARUVI_WEBHOOK_HOST:'127.0.0.1', ARUVI_WEBHOOK_PORT:String(port), ARUVI_MOBILE_API_TOKEN:token, ARUVI_MCP_API_TOKEN:token } });
  for (let i = 0;; i++) { if (server.exitCode !== null || i === 120) throw Error('Server did not start'); try { if ((await fetch(base + '/health')).ok) break; } catch {} await new Promise(done => setTimeout(done, 250)); }
  const headers = { 'Content-Type':'application/json', Authorization:`Bearer ${token}` };
  async function call(name, action, args) {
    const response = await fetch(base + '/api/mcp', { method:'POST', headers, body:JSON.stringify({ jsonrpc:'2.0', id:1, method:'tools/call', params:{name, arguments:{action, arguments:args}} }) });
    const result = (await response.json()).result; assert(!result.isError, JSON.stringify(result)); return result.structuredContent.result;
  }
  const product = await call('aruvi_catalog', 'create_product', { name:'Findings fixture' });
  const task = await call('aruvi_work_items', 'create_work_item', { product_id:product.id, title:'Review findings task', work_item_type:'task' });
  const other = await call('aruvi_work_items', 'create_work_item', { product_id:product.id, title:'Other findings task', work_item_type:'task' });
  execFileSync('python3', ['-c', `import sqlite3,sys
c=sqlite3.connect(sys.argv[1])
c.execute("INSERT INTO findings(id,work_item_id,category,severity,title,description,status,is_blocking) VALUES(?,?,?,?,?,?,?,?)",('finding-a',sys.argv[2],'security','high','Unsafe <img src=x onerror=alert(1)>','Review the boundary','open',1))
c.execute("INSERT INTO findings(id,work_item_id,category,severity,title,description,status,is_blocking) VALUES(?,?,?,?,?,?,?,?)",('finding-b',sys.argv[3],'performance','low','Other task finding','Private to other task','resolved',0))
c.commit()`, db, task.id, other.id]);
  assert.equal((await fetch(base + `/api/mobile/work-items/${task.id}/delivery`)).status, 401);
  const delivery = await (await fetch(base + `/api/mobile/work-items/${task.id}/delivery`, { headers })).json();
  const findings = await call('aruvi_checkpoints', 'list_work_item_findings', { work_item_id:task.id });
  assert.deepEqual(delivery.findings, findings); assert.deepEqual(findings.map(item => item.id), ['finding-a']);
  browser = await chromium.launch({ executablePath:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless:true });
  const page = await browser.newPage(); let dialogOpened = false; page.on('dialog', dialog => { dialogOpened = true; void dialog.dismiss(); });
  await page.goto(base + '/remote'); await page.locator('#tokenInput').fill(token); await page.locator('#saveButton').click();
  await page.getByRole('button', { name:'Work', exact:true }).click(); await page.locator('#workProductSelect').selectOption(product.id);
  await page.getByText('Review findings task', { exact:false }).first().click();
  await page.getByText('Unsafe <img src=x onerror=alert(1)>', { exact:true }).waitFor();
  const text = await page.locator('#deliveryDetails').innerText();
  assert(text.includes('high · security · open · blocking')); assert(text.includes('Review the boundary')); assert(!text.includes('Other task finding'));
  assert.equal(await page.locator('#deliveryDetails img').count(), 0); assert.equal(dialogOpened, false);
  console.log('Native browser findings, safe rendering, authorization and MCP isolation passed.');
} finally {
  await browser?.close();
  if (server && server.exitCode === null) { server.kill(); await new Promise(done => server.once('exit', done)); }
  await rm(root, { recursive:true, force:true });
}
