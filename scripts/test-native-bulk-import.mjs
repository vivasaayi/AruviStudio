// Disposable server/database: validate browser and MCP inline catalog imports.
import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn } from 'node:child_process';
import net from 'node:net';
import { chromium } from '@playwright/test';
const root = await mkdtemp(join(tmpdir(),'aruvi-inline-import-'));
let server, browser;
try {
  const port = await new Promise(done => { const s=net.createServer(); s.listen(0,'127.0.0.1',() => { const p=s.address().port; s.close(() => done(p)); }); });
  const token='inline-import-fixture-token', base=`http://127.0.0.1:${port}`;
  server=spawn(resolve('src-tauri/target/debug/aruvi-studio-server'),[],{stdio:'ignore',env:{...process.env,ARUVI_APP_DATA_DIR:root,ARUVI_DB_PATH:join(root,'db.sqlite'),ARUVI_WEBHOOK_HOST:'127.0.0.1',ARUVI_WEBHOOK_PORT:String(port),ARUVI_MOBILE_API_TOKEN:token,ARUVI_MCP_API_TOKEN:token}});
  for (let i=0;;i++) { if (server.exitCode !== null || i === 120) throw new Error('Native fixture did not start'); try { if ((await fetch(base+'/health')).ok) break; } catch {} await new Promise(r => setTimeout(r,250)); }
  const headers={'Content-Type':'application/json',Authorization:`Bearer ${token}`};
  async function rpc(action,args) {
    const response=await fetch(base+'/api/mcp',{method:'POST',headers:{...headers,Accept:'application/json, text/event-stream'},body:JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/call',params:{name:'aruvi_catalog',arguments:{action,arguments:args}}})});
    return (await response.json()).result;
  }
  async function waitJob(id) {
    for (let i=0;i<100;i++) { const response=await fetch(base+'/api/mobile/bulk-import/'+encodeURIComponent(id),{headers}); assert.equal(response.status,200); const result=await response.json(); if (['completed','failed'].includes(result.job.status)) return result; await new Promise(r => setTimeout(r,100)); }
    throw new Error('Import did not finish');
  }
  const missing=await fetch(base+'/api/mobile/bulk-import'); assert.equal(missing.status,401);
  const unauthenticated=await fetch(base+'/api/mobile/bulk-import',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({content:'{}',format:'json'})}); assert.equal(unauthenticated.status,401);
  const invalid=await fetch(base+'/api/mobile/bulk-import',{method:'POST',headers,body:JSON.stringify({content:'x'.repeat(1048577),format:'json'})}); assert.equal(invalid.status,400);
  const blank=await rpc('submit_inline_bulk_import',{content:' ',format:'json'}); assert.equal(blank.isError,true);
  const oversized=await rpc('submit_inline_bulk_import',{content:'x'.repeat(1048577),format:'json'}); assert.equal(oversized.isError,true);
  const badFormat=await rpc('submit_inline_bulk_import',{content:'{}',format:'yaml'}); assert.equal(badFormat.isError,true);
  const mcp=await rpc('submit_inline_bulk_import',{format:'json',content:JSON.stringify({product:{id:'mcp-import-product',name:'MCP import'},productAreas:[{id:'mcp-import-area',name:'MCP area'}]})});
  assert(!mcp.isError,JSON.stringify(mcp)); const mcpJob=mcp.structuredContent.result; assert.equal(mcpJob.sourcePath,'<inline>');
  const complete=await waitJob(mcpJob.id); assert.equal(complete.job.status,'completed',JSON.stringify(complete)); assert.equal(complete.job.productCount,1); assert.equal(complete.job.productAreaCount,1);
  const update=await rpc('submit_inline_bulk_import',{format:'json',content:JSON.stringify({product:{id:'mcp-import-product',name:'Updated MCP import'},productAreas:[]})});
  assert(!update.isError,JSON.stringify(update)); assert.equal((await waitJob(update.structuredContent.result.id)).job.status,'completed');
  const path=join(root,'legacy.csv'); await writeFile(path,'record_type,id,name\nproduct,legacy-import-product,Legacy import\n');
  const legacy=await rpc('submit_bulk_import',{file_path:path}); assert(!legacy.isError,JSON.stringify(legacy));
  assert.equal((await waitJob(legacy.structuredContent.result.id)).job.status,'completed');
  const bad=await rpc('submit_inline_bulk_import',{format:'json',content:'{invalid'}); const failed=await waitJob(bad.structuredContent.result.id); assert.equal(failed.job.status,'failed'); assert(failed.errors.length>0);
  browser=await chromium.launch({executablePath:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const page=await browser.newPage(); await page.goto(base+'/remote'); await page.locator('#tokenInput').fill(token); await page.locator('#saveButton').click();
  await page.getByRole('button',{name:'Products',exact:true}).click(); await page.getByText('Import JSON or CSV',{exact:true}).click();
  const json=JSON.stringify({product:{id:'browser-import-product',name:'Browser import'},productAreas:[{id:'browser-import-area',name:'Browser area'}]});
  await page.locator('#bulkImportFile').setInputFiles({name:'import.json',mimeType:'application/json',buffer:Buffer.from(json)});
  page.once('dialog',dialog => dialog.accept());
  const submitted=page.waitForResponse(response => response.url().endsWith('/api/mobile/bulk-import') && response.request().method()==='POST');
  await page.getByRole('button',{name:'Start import'}).click(); const result=await submitted; assert.equal(result.status(),202); const job=await result.json();
  const browserDone=await waitJob(job.id); assert.equal(browserDone.job.status,'completed',JSON.stringify(browserDone));
  await page.locator('#bulkImportFormat').selectOption('csv');
  await page.locator('#bulkImportFile').setInputFiles({name:'import.csv',mimeType:'text/csv',buffer:Buffer.from('record_type,id,name\nproduct,csv-import-product,CSV import\n')});
  page.once('dialog',dialog => dialog.accept());
  const csvResponse=page.waitForResponse(response => response.url().endsWith('/api/mobile/bulk-import') && response.request().method()==='POST');
  await page.getByRole('button',{name:'Start import'}).click(); const csvJob=await (await csvResponse).json();
  const csvDone=await waitJob(csvJob.id); assert.equal(csvDone.job.status,'completed',JSON.stringify(csvDone));
  await page.getByRole('button',{name:'Refresh imports'}).click();
  await page.getByRole('button',{name:/JSON — completed/}).first().waitFor();
  const products=await rpc('list_products',{}); const list=products.structuredContent.result; assert(list.some(item => item.id==='browser-import-product')); assert(list.some(item => item.id==='mcp-import-product' && item.name==='Updated MCP import')); assert(list.some(item => item.id==='csv-import-product'));
  const env={...process.env,ARUVI_REMOTE_URL:base,ARUVI_TEST_INSPECT:'1',ARUVI_MOBILE_API_TOKEN:token,ARUVI_MCP_API_TOKEN:token,ARUVI_CHROME_PATH:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'};
  const suite=spawn(resolve('node_modules/.bin/playwright'),['test','-c','playwright.remote.config.js','remote.spec.js','-g','browser and MCP import JSON and CSV'],{stdio:'inherit',env});
  const suiteStatus=await new Promise((done,reject) => { suite.once('error',reject); suite.once('exit',(code,signal) => done({code,signal})); });
  if (suiteStatus.code !== 0) throw new Error(`Remote browser test failed: ${suiteStatus.code ?? suiteStatus.signal}`);
  console.log('Native browser and MCP inline imports passed, including authorization, size limits, failure status and persisted product records.');
} finally { await browser?.close(); if (server && server.exitCode===null && server.signalCode===null) { server.kill(); await new Promise(r => server.once('exit',r)); } await rm(root,{recursive:true,force:true}); }
