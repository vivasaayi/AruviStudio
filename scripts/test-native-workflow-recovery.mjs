// Exercise the browser against a disposable native server and synthetic stuck run.
import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn, execFileSync } from 'node:child_process';
import net from 'node:net';
import { chromium } from '@playwright/test';

const root = await mkdtemp(join(tmpdir(), 'aruvi-workflow-recovery-'));
let server, browser;
try {
  const port = await new Promise(done => { const socket = net.createServer(); socket.listen(0, '127.0.0.1', () => { const value = socket.address().port; socket.close(() => done(value)); }); });
  const base = `http://127.0.0.1:${port}`, token = 'workflow-recovery-fixture', db = join(root, 'db.sqlite');
  server = spawn(resolve('src-tauri/target/debug/aruvi-studio-server'), [], { stdio:'ignore', env:{ ...process.env, ARUVI_APP_DATA_DIR:root, ARUVI_DB_PATH:db, ARUVI_WEBHOOK_HOST:'127.0.0.1', ARUVI_WEBHOOK_PORT:String(port), ARUVI_MOBILE_API_TOKEN:token, ARUVI_MCP_API_TOKEN:token } });
  for (let i = 0;; i++) { if (server.exitCode !== null || i === 120) throw Error('Server did not start'); try { if ((await fetch(base + '/health')).ok) break; } catch {} await new Promise(done => setTimeout(done, 250)); }
  const headers = { 'Content-Type':'application/json', Authorization:`Bearer ${token}` };
  async function call(name, action, args) {
    const response = await fetch(base + '/api/mcp', { method:'POST', headers, body:JSON.stringify({ jsonrpc:'2.0', id:1, method:'tools/call', params:{name, arguments:{action, arguments:args}} }) });
    const result = (await response.json()).result; assert(!result.isError, JSON.stringify(result)); return result.structuredContent.result;
  }
  const product = await call('aruvi_catalog', 'create_product', { name:'Workflow recovery fixture' });
  const task = await call('aruvi_work_items', 'create_work_item', { product_id:product.id, title:'Recovery fixture task', work_item_type:'task' });
  execFileSync('python3', ['-c', 'import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute("INSERT INTO workflow_runs(id,work_item_id,status,current_stage) VALUES(?,?,?,?)",("recovery",sys.argv[2],"running","coding")); c.commit()', db, task.id]);
  browser = await chromium.launch({ executablePath:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless:true });
  const page = await browser.newPage();
  await page.goto(base + '/remote'); await page.locator('#tokenInput').fill(token); await page.locator('#saveButton').click();
  await page.getByRole('button', { name:'Work', exact:true }).click(); await page.locator('#workProductSelect').selectOption(product.id);
  await page.getByText('Recovery fixture task', { exact:false }).first().click();
  const button = page.getByRole('button', { name:'Mark workflow failed', exact:true }); await button.waitFor();
  assert((await page.locator('#deliveryDetails').innerText()).includes('does not stop running processes'));
  page.once('dialog', dialog => dialog.dismiss()); await button.click();
  let run = await call('aruvi_workflows', 'get_workflow_run', { workflow_run_id:'recovery' }); assert.equal(run.status, 'running');
  const onDialog = async dialog => { if (dialog.type() === 'prompt') await dialog.accept('Fixture recovery'); else await dialog.accept(); };
  page.on('dialog', onDialog);
  const response = page.waitForResponse(r => r.url().endsWith('/workflows/recovery/mark-failed'));
  await button.click(); assert.equal((await response).status(), 200); page.off('dialog', onDialog);
  await page.locator('#deliveryStatus').getByText('failed').waitFor();
  run = await call('aruvi_workflows', 'get_workflow_run', { workflow_run_id:'recovery' });
  assert.equal(run.status, 'failed'); assert.equal(run.current_stage, 'failed'); assert.equal(run.error_message, 'Fixture recovery');
  const history = await call('aruvi_workflows', 'get_workflow_history', { workflow_run_id:'recovery' });
  assert.equal(history.filter(entry => entry.trigger === 'user_override' && entry.to_stage === 'failed').length, 1);
  console.log('Native browser recovery confirmation, cancellation and MCP history passed.');
} finally {
  await browser?.close();
  if (server && server.exitCode === null) { server.kill(); await new Promise(done => server.once('exit', done)); }
  await rm(root, { recursive:true, force:true });
}
