// Disposable native server and simulated model; no existing Studio data is used.
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn, execFileSync } from 'node:child_process';
import http from 'node:http';
import net from 'node:net';
import { chromium } from '@playwright/test';
const root = await mkdtemp(join(tmpdir(), 'aruvi-general-copilot-'));
const token = 'general-copilot-fixture-token';
let server, browser;
const captured = [];
const mock = http.createServer((req, res) => {
  let data = ''; req.on('data', chunk => data += chunk);
  req.on('end', () => {
    const body = JSON.parse(data); captured.push(body);
    if (body.messages.some(m => m.role === 'system' && m.content.includes("type 'patch_proposal'"))) {
      const content = JSON.stringify({type:'patch_proposal',summary:'Update fixture',patches:[{path:'fixture.txt',description:'Change content',patch:'@@ -1,1 +1,1 @@\n-fixture content\n+updated content'}]});
      if (body.stream) { res.writeHead(200, {'content-type':'text/event-stream'}); res.end('data: '+JSON.stringify({choices:[{delta:{content},finish_reason:'stop'}]})+'\n\ndata: [DONE]\n\n'); }
      else { res.writeHead(200, {'content-type':'application/json'}); res.end(JSON.stringify({choices:[{message:{role:'assistant',content},finish_reason:'stop'}]})); }
    } else if (body.stream) { res.writeHead(200, {'content-type':'text/event-stream'}); res.end('data: {"choices":[{"delta":{"content":"streamed answer"},"finish_reason":"stop"}]}\n\ndata: [DONE]\n\n'); }
    else { res.writeHead(200, {'content-type':'application/json'}); res.end(JSON.stringify({choices:[{message:{role:'assistant',content:'model answer'},finish_reason:'stop'}]})); }
  });
});
try {
  await new Promise(resolve => mock.listen(0, '127.0.0.1', resolve));
  const port = await new Promise(resolve => { const s = net.createServer(); s.listen(0,'127.0.0.1', () => { const p=s.address().port; s.close(() => resolve(p)); }); });
  const base = `http://127.0.0.1:${port}`;
  server = spawn(resolve('src-tauri/target/debug/aruvi-studio-server'), [], {stdio:'ignore', env:{...process.env, ARUVI_APP_DATA_DIR:root, ARUVI_DB_PATH:join(root,'db.sqlite'), ARUVI_WEBHOOK_HOST:'127.0.0.1', ARUVI_WEBHOOK_PORT:String(port), ARUVI_MOBILE_API_TOKEN:token, ARUVI_MCP_API_TOKEN:token}});
  for (let i=0;;i++) { if (server.exitCode !== null || i === 120) throw new Error('Native fixture did not start'); try { if ((await fetch(base+'/health')).ok) break; } catch {} await new Promise(r => setTimeout(r, 250)); }
  async function call(name, action, args) {
    const rpc = await fetch(base+'/api/mcp', {method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${token}`,Accept:'application/json, text/event-stream'},body:JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/call',params:{name,arguments:{action,arguments:args}}})});
    const data = await rpc.json(); assert(!data.result.isError,JSON.stringify(data)); return data.result.structuredContent.result;
  }
  const provider = (await call('aruvi_models','create_provider',{name:'General fixture',base_url:`http://127.0.0.1:${mock.address().port}/v1`})).id;
  const baseRequest = {provider_id:provider, model_name:'test-model', messages:[{role:'user',content:'hello'}]};
  for (const invalid of [
    {...baseRequest, max_tokens:32769},
    {...baseRequest, temperature:2.1},
    {...baseRequest, messages:[{role:'tool',content:'not supported'}]},
    {...baseRequest, messages:[{role:'user',content:'x'.repeat(256001)}]},
  ]) {
    for (const path of ['/api/mobile/ide/copilot','/api/mobile/ide/copilot/stream']) {
      const response = await fetch(base+path,{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${token}`},body:JSON.stringify(invalid)});
      assert.equal(response.status,400);
    }
    const rpc = await fetch(base+'/api/mcp',{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${token}`,Accept:'application/json, text/event-stream'},body:JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/call',params:{name:'aruvi_models',arguments:{action:'run_general_copilot',arguments:invalid}}})});
    assert((await rpc.json()).result.isError);
  }
  assert.equal(captured.length,0,'Invalid general Copilot requests reached the provider');
  const repo = join(root,'repo'); await mkdir(repo); execFileSync('git',['init','-q',repo]); await writeFile(join(repo,'fixture.txt'),'fixture content\n');
  await call('aruvi_repositories','register_repository',{name:'General workspace',local_path:repo});
  browser = await chromium.launch({executablePath:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const page = await browser.newPage(); await page.goto(base+'/remote');
  await page.locator('#tokenInput').fill(token); await page.locator('#providerInput').fill(provider); await page.locator('#modelInput').fill('test-model'); await page.locator('#saveButton').click();
  await page.getByRole('button',{name:'Workspaces',exact:true}).click();
  await page.locator('#repositoryCopilotQuestion').fill('Explain ownership'); await page.locator('#repositoryCopilotButton').click();
  await page.getByText('You: Explain ownership', {exact:false}).waitFor();
  assert((await page.locator('#repositoryCopilotResult').textContent()).includes('model answer'));
  await page.locator('#repositoryCopilotStream').check(); await page.locator('#repositoryCopilotQuestion').fill('Give an example'); await page.locator('#repositoryCopilotButton').click();
  await page.getByText('streamed answer',{exact:false}).waitFor();
  assert.deepEqual(captured[1].messages.slice(-3).map(m => m.content), ['Explain ownership','model answer','Give an example']);
  await page.locator('#repositoryList .tool-card').filter({hasText:'General workspace'}).getByRole('button',{name:'Browse workspace'}).click();
  await page.locator('#repositoryTree').getByRole('button',{name:'fixture.txt'}).click();
  await page.getByRole('button',{name:'Close fixture.txt'}).waitFor(); assert.equal(await page.locator('#repositoryCopilotResult').textContent(),'');
  await page.getByRole('button',{name:'Close fixture.txt'}).click();
  assert((await page.locator('#repositoryCopilotResult').textContent()).includes('streamed answer'));
  await page.locator('#repositoryCopilotStream').uncheck();
  await page.locator('#repositoryCopilotQuestion').fill('Update fixture content');
  await page.locator('#repositoryCopilotPatchButton').click();
  await page.locator('#repositoryPatchApply:not([disabled])').waitFor();
  await writeFile(join(repo,'fixture.txt'),'changed after review\n');
  await page.locator('#repositoryPatchApply').click();
  await page.getByText(/Applied 0 of 1 patches; stopped/).waitFor();
  assert.equal(await readFile(join(repo,'fixture.txt'),'utf8'),'changed after review\n');
  await writeFile(join(repo,'fixture.txt'),'fixture content\n');
  await page.locator('#repositoryCopilotStream').check();
  await page.locator('#repositoryCopilotQuestion').fill('Update fixture content');
  await page.locator('#repositoryCopilotPatchButton').click();
  await page.locator('#repositoryPatchApply:not([disabled])').waitFor();
  await page.locator('#repositoryPatchApply').click();
  await page.getByText('Applied 1 reviewed patch(es)',{exact:true}).waitFor();
  assert.equal(await readFile(join(repo,'fixture.txt'),'utf8'),'updated content\n');
  await page.locator('#repositoryCopilotStream').uncheck();
  await writeFile(join(repo,'fixture.txt'),'fixture content\n');
  await page.locator('#repositoryTree').getByRole('button',{name:'fixture.txt'}).click();
  await page.getByRole('button',{name:'Close fixture.txt'}).waitFor();
  await page.locator('#repositoryCopilotQuestion').fill('Update the open file');
  await page.locator('#repositoryCopilotPatchButton').click();
  await page.locator('#repositoryPatchApply:not([disabled])').waitFor();
  await page.locator('#repositoryPatchApply').click();
  await page.getByText('Applied 1 reviewed patch(es)',{exact:true}).waitFor();
  assert.equal(await readFile(join(repo,'fixture.txt'),'utf8'),'updated content\n');
  const noAuth = await fetch(base+'/api/mobile/ide/copilot',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({messages:[{role:'user',content:'x'}]})}); assert.equal(noAuth.status,401);
  console.log('Native browser and authenticated general Copilot endpoints passed with simulated model, including streaming and history.');
} finally { await browser?.close(); if (server && server.exitCode === null && server.signalCode === null) { server.kill(); await new Promise(r => server.once('exit',r)); } mock.close(); await rm(root,{recursive:true,force:true}); }
