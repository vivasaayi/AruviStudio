// Disposable server and SQLite database: protect open buffers from workspace path changes.
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn, execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import net from 'node:net';
import { chromium } from '@playwright/test';
const root=await mkdtemp(join(tmpdir(),'aruvi-workspace-root-'));
let server,browser,writer;
try {
  const first=join(root,'first'), second=join(root,'second');
  for (const dir of [first,second]) { await mkdir(dir); execFileSync('git',['init','-q',dir]); await writeFile(join(dir,'fixture.txt'),'same content\n'); }
  const firstRoot=await realpath(first), secondRoot=await realpath(second);
  const port=await new Promise(done => { const s=net.createServer(); s.listen(0,'127.0.0.1',() => { const p=s.address().port; s.close(() => done(p)); }); });
  const base=`http://127.0.0.1:${port}`, token='workspace-root-fixture-token', db=join(root,'db.sqlite');
  server=spawn(resolve('src-tauri/target/debug/aruvi-studio-server'),[],{stdio:'ignore',env:{...process.env,ARUVI_APP_DATA_DIR:root,ARUVI_DB_PATH:db,ARUVI_WEBHOOK_HOST:'127.0.0.1',ARUVI_WEBHOOK_PORT:String(port),ARUVI_MOBILE_API_TOKEN:token,ARUVI_MCP_API_TOKEN:token}});
  for (let i=0;;i++) { if (server.exitCode!==null || i===120) throw new Error('Native fixture did not start'); try { if ((await fetch(base+'/health')).ok) break; } catch {} await new Promise(r=>setTimeout(r,250)); }
  const headers={'Content-Type':'application/json',Authorization:`Bearer ${token}`};
  const rpc=await fetch(base+'/api/mcp',{method:'POST',headers:{...headers,Accept:'application/json, text/event-stream'},body:JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/call',params:{name:'aruvi_repositories',arguments:{action:'register_repository',arguments:{name:'Guard workspace',local_path:first}}}})});
  const body=await rpc.json(); assert(!body.result.isError,JSON.stringify(body)); const repo=body.result.structuredContent.result; assert.equal(repo.local_path,firstRoot);
  browser=await chromium.launch({executablePath:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const page=await browser.newPage(); await page.goto(base+'/remote'); await page.locator('#tokenInput').fill(token); await page.locator('#saveButton').click();
  await page.getByRole('button',{name:'Workspaces',exact:true}).click();
  await page.locator('#repositoryList .tool-card').filter({hasText:'Guard workspace'}).getByRole('button',{name:'Browse workspace'}).click();
  await page.locator('#repositoryTree').getByRole('button',{name:'fixture.txt',exact:true}).click();
  await page.getByLabel('Selected file contents').fill('browser edit\n');
  // The other process takes SQLite's write lock, changes the workspace path,
  // then commits while the browser save is waiting for that lock.
  writer=spawn('python3',['-u','-c','import sqlite3,sys,time\nc=sqlite3.connect(sys.argv[1]); c.execute("BEGIN IMMEDIATE"); c.execute("UPDATE repositories SET local_path=? WHERE id=?",(sys.argv[2],sys.argv[3])); print("locked",flush=True); time.sleep(1); c.commit()',db,secondRoot,repo.id],{stdio:['ignore','pipe','pipe']});
  await new Promise((done,reject) => { writer.stdout.once('data',done); writer.once('error',reject); writer.once('exit',code => { if (code) reject(new Error('Workspace writer failed')); }); });
  const pending=page.waitForResponse(response => response.url().endsWith('/api/mobile/repository-file/save'));
  await page.getByRole('button',{name:'Save file',exact:true}).click(); const response=await pending; assert.equal(response.status(),409);
  await page.locator('#status').getByText(/Workspace path changed/).waitFor();
  assert.equal(await page.getByLabel('Selected file contents').inputValue(),'browser edit\n');
  assert.equal(await readFile(join(first,'fixture.txt'),'utf8'),'same content\n'); assert.equal(await readFile(join(second,'fixture.txt'),'utf8'),'same content\n');
  async function call(action,args) {
    const response=await fetch(base+'/api/mcp',{method:'POST',headers:{...headers,Accept:'application/json, text/event-stream'},body:JSON.stringify({jsonrpc:'2.0',id:2,method:'tools/call',params:{name:'aruvi_repositories',arguments:{action,arguments:args}}})});
    return (await response.json()).result;
  }
  const read=await call('read_repository_file',{repository_id:repo.id,relative_path:'fixture.txt'});
  assert.equal(read.structuredContent.result.workspace_root,secondRoot);
  const staleWrite=await call('write_repository_file',{repository_id:repo.id,relative_path:'fixture.txt',expected_root:firstRoot,content:'wrong MCP destination\n'});
  assert.equal(staleWrite.isError,true); assert.equal(await readFile(join(second,'fixture.txt'),'utf8'),'same content\n');
  const sha=createHash('sha256').update('same content\n').digest('hex');
  const patch='@@ -1,1 +1,1 @@\n-same content\n+patched content';
  const stalePatch=await call('apply_reviewed_repository_patch',{repository_id:repo.id,relative_path:'fixture.txt',expected_root:firstRoot,patch,base_sha256:sha,expected_exists:true});
  assert.equal(stalePatch.isError,true); assert.equal(await readFile(join(second,'fixture.txt'),'utf8'),'same content\n');
  const staleLegacyPatch=await call('apply_repository_patch',{repository_id:repo.id,relative_path:'fixture.txt',expected_root:firstRoot,patch,base_sha256:sha});
  assert.equal(staleLegacyPatch.isError,true);
  const apply=await fetch(base+'/api/mobile/repository-inspect',{method:'POST',headers,body:JSON.stringify({action:'apply_reviewed_repository_patch',repository_id:repo.id,expected_root:firstRoot,relative_path:'fixture.txt',patch,base_sha256:sha,expected_exists:true})});
  assert.equal(apply.status,409); assert.equal(await readFile(join(second,'fixture.txt'),'utf8'),'same content\n');
  const good=await fetch(base+'/api/mobile/repository-inspect',{method:'POST',headers,body:JSON.stringify({action:'apply_reviewed_repository_patch',repository_id:repo.id,expected_root:secondRoot,relative_path:'fixture.txt',patch,base_sha256:sha,expected_exists:true})});
  assert.equal(good.status,200,await good.text()); assert.equal(await readFile(join(second,'fixture.txt'),'utf8'),'patched content\n');
  const legacy=await fetch(base+'/api/mobile/repository-file/save',{method:'POST',headers,body:JSON.stringify({repository_id:repo.id,relative_path:'fixture.txt',expected_content:'patched content\n',content:'legacy edit\n'})});
  assert.equal(legacy.status,200,await legacy.text()); assert.equal(await readFile(join(second,'fixture.txt'),'utf8'),'legacy edit\n');
  const validWrite=await call('write_repository_file',{repository_id:repo.id,relative_path:'fixture.txt',expected_root:secondRoot,content:'valid MCP edit\n'});
  assert(!validWrite.isError,JSON.stringify(validWrite)); assert.equal(await readFile(join(second,'fixture.txt'),'utf8'),'valid MCP edit\n');
  writer=spawn('python3',['-u','-c','import sqlite3,sys,time\nc=sqlite3.connect(sys.argv[1]); c.execute("BEGIN IMMEDIATE"); c.execute("UPDATE repositories SET local_path=? WHERE id=?",(sys.argv[2],sys.argv[3])); print("locked",flush=True); time.sleep(1); c.commit()',db,firstRoot,repo.id],{stdio:['ignore','pipe','pipe']});
  await new Promise((done,reject) => { writer.stdout.once('data',done); writer.once('error',reject); writer.once('exit',code => { if (code) reject(new Error('Workspace writer failed')); }); });
  const waitingWrite=await call('write_repository_file',{repository_id:repo.id,relative_path:'fixture.txt',expected_root:secondRoot,content:'raced MCP edit\n'});
  assert.equal(waitingWrite.isError,true); assert.equal(await readFile(join(first,'fixture.txt'),'utf8'),'same content\n'); assert.equal(await readFile(join(second,'fixture.txt'),'utf8'),'valid MCP edit\n');
  console.log('Cross-process workspace changes rejected stale browser and MCP writes and patches; matching path writes passed.');
} finally { if (writer && writer.exitCode===null && writer.signalCode===null) writer.kill(); await browser?.close(); if (server && server.exitCode===null && server.signalCode===null) { server.kill(); await new Promise(r=>server.once('exit',r)); } await rm(root,{recursive:true,force:true}); }
