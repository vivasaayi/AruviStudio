// Disposable server/database: exercise workspace edits across MCP and browser.
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn, execFileSync } from 'node:child_process';
import net from 'node:net';
import { chromium } from '@playwright/test';
const root=await mkdtemp(join(tmpdir(),'aruvi-workspace-edit-'));
let server,browser;
try {
  const paths=[]; for (const name of ['first','other','replacement','race']) { const dir=join(root,name); await mkdir(dir); execFileSync('git',['init','-q',dir]); await writeFile(join(dir,'fixture.txt'),'same content\n'); paths.push(await realpath(dir)); }
  const port=await new Promise(done => { const s=net.createServer(); s.listen(0,'127.0.0.1',() => {const p=s.address().port;s.close(()=>done(p));});});
  const base=`http://127.0.0.1:${port}`, token='workspace-edit-fixture-token', db=join(root,'db.sqlite');
  server=spawn(resolve('src-tauri/target/debug/aruvi-studio-server'),[],{stdio:'ignore',env:{...process.env,ARUVI_APP_DATA_DIR:root,ARUVI_DB_PATH:db,ARUVI_WEBHOOK_HOST:'127.0.0.1',ARUVI_WEBHOOK_PORT:String(port),ARUVI_MOBILE_API_TOKEN:token,ARUVI_MCP_API_TOKEN:token}});
  for (let i=0;;i++) { if (server.exitCode!==null || i===120) throw new Error('Native fixture did not start'); try { if ((await fetch(base+'/health')).ok) break; } catch {} await new Promise(r=>setTimeout(r,250)); }
  const headers={'Content-Type':'application/json',Authorization:`Bearer ${token}`};
  async function call(tool,action,args) {
    const response=await fetch(base+'/api/mcp',{method:'POST',headers:{...headers,Accept:'application/json, text/event-stream'},body:JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/call',params:{name:tool,arguments:{action,arguments:args}}})}); return (await response.json()).result;
  }
  const result=async (tool,action,args) => {const reply=await call(tool,action,args); assert(!reply.isError,JSON.stringify(reply)); return reply.structuredContent.result;};
  const repo=await result('aruvi_repositories','register_repository',{name:'Edit workspace',local_path:paths[0],default_branch:'main'});
  await result('aruvi_repositories','register_repository',{name:'Other workspace',local_path:paths[1],default_branch:'main'});
  const product=await result('aruvi_catalog','create_product',{name:'Workspace edit fixture'});
  const task1=await result('aruvi_work_items','create_work_item',{product_id:product.id,title:'Inherited branch',work_item_type:'task'});
  const task2=await result('aruvi_work_items','create_work_item',{product_id:product.id,title:'Custom branch',work_item_type:'task'});
  const py=(program,...args)=>execFileSync('python3',['-c',program,db,...args],{encoding:'utf8'}).trim();
  py('import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]); c.execute("UPDATE work_items SET active_repo_id=?,branch_name=? WHERE id=?",(sys.argv[2],"main",sys.argv[3])); c.execute("UPDATE work_items SET active_repo_id=?,branch_name=? WHERE id=?",(sys.argv[2],"custom",sys.argv[4])); c.commit()',repo.id,task1.id,task2.id);
  const update={id:repo.id,name:'MCP updated',local_path:paths[0],remote_url:'',default_branch:'develop',expected:repo};
  assert((await call('aruvi_repositories','update_repository',{...update,expected:undefined})).isError);
  assert((await call('aruvi_repositories','update_repository',{...update,local_path:paths[1]})).isError);
  py('import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]); c.execute("INSERT INTO workflow_runs (id,work_item_id,status,current_stage) VALUES (?,?,?,?)",("active-fixture",sys.argv[2],"running","coding")); c.commit()',task1.id);
  assert((await call('aruvi_repositories','update_repository',update)).isError);
  py('import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]); c.execute("UPDATE workflow_runs SET status=\'paused\' WHERE id=\'active-fixture\'"); c.commit()');
  assert((await call('aruvi_repositories','update_repository',update)).isError);
  py('import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]); c.execute("UPDATE workflow_runs SET status=\'completed\',current_stage=\'done\' WHERE id=\'active-fixture\'"); c.execute("INSERT INTO external_cli_runs (id,work_item_id,provider,label,command,status) VALUES (?,?,?,?,?,?)",("cli-fixture",sys.argv[2],"fixture","fixture","fixture","running")); c.commit()',task1.id);
  assert((await call('aruvi_repositories','update_repository',update)).isError);
  py('import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]); c.execute("UPDATE external_cli_runs SET status=\'completed\' WHERE id=\'cli-fixture\'"); c.commit()');
  // A database failure during inherited branch propagation must roll back the repository edit.
  py('import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]); c.execute("CREATE TRIGGER fail_branch BEFORE UPDATE OF branch_name ON work_items BEGIN SELECT RAISE(ABORT,\'fixture\'); END"); c.commit()');
  assert((await call('aruvi_repositories','update_repository',update)).isError);
  const unchanged=(await result('aruvi_repositories','list_repositories',{})).find(item=>item.id===repo.id); assert.equal(unchanged.name,repo.name); assert.equal(unchanged.default_branch,'main');
  py('import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]); c.execute("DROP TRIGGER fail_branch"); c.commit()');
  const updated=await result('aruvi_repositories','update_repository',update); assert.equal(updated.default_branch,'develop');
  const branches=JSON.parse(py('import sqlite3,sys,json\nc=sqlite3.connect(sys.argv[1]); print(json.dumps([c.execute("SELECT branch_name FROM work_items WHERE id=?",(x,)).fetchone()[0] for x in sys.argv[2:]]))',task1.id,task2.id)); assert.deepEqual(branches,['develop','custom']);
  assert((await call('aruvi_repositories','update_repository',update)).isError);
  browser=await chromium.launch({executablePath:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const page=await browser.newPage(); await page.goto(base+'/remote'); await page.locator('#tokenInput').fill(token); await page.locator('#saveButton').click();
  await page.getByRole('button',{name:'Workspaces',exact:true}).click();
  let card=page.locator('#repositoryList .tool-card').filter({hasText:'MCP updated'}); await card.getByText('Edit workspace',{exact:true}).click();
  assert.equal(await page.getByLabel('Workspace name',{exact:true}).count(),1);
  await card.getByLabel('Edit workspace name').fill('Browser draft');
  await result('aruvi_repositories','update_repository',{...update,name:'Concurrent MCP',default_branch:'develop',expected:updated});
  const stale=page.waitForResponse(response=>response.url().includes('/api/mobile/repositories/') && response.request().method()==='POST');
  await card.getByRole('button',{name:'Save workspace'}).click(); assert.equal((await stale).status(),409); assert.equal(await card.getByLabel('Edit workspace name').inputValue(),'Browser draft');
  await page.locator('#refreshWorkspacesButton').click(); card=page.locator('#repositoryList .tool-card').filter({hasText:'Concurrent MCP'});
  await card.getByRole('button',{name:'Browse workspace'}).click(); await page.locator('#repositoryTree').getByRole('button',{name:'fixture.txt',exact:true}).click();
  await page.getByLabel('Selected file contents').fill('unsaved browser edit\n');
  await card.getByText('Edit workspace',{exact:true}).click();
  await card.getByLabel('Edit workspace name').fill('Browser updated'); await card.getByLabel('Server path').fill(paths[2]);
  page.once('dialog',dialog=>dialog.accept());
  const saved=page.waitForResponse(response=>response.url().includes('/api/mobile/repositories/') && response.request().method()==='POST');
  await card.getByRole('button',{name:'Save workspace'}).click(); assert.equal((await saved).status(),200);
  const staleFile=page.waitForResponse(response=>response.url().endsWith('/api/mobile/repository-file/save'));
  await page.getByRole('button',{name:'Save file',exact:true}).click(); assert.equal((await staleFile).status(),409);
  assert.equal(await page.getByLabel('Selected file contents').inputValue(),'unsaved browser edit\n');
  assert.equal(await readFile(join(paths[0],'fixture.txt'),'utf8'),'same content\n');
  const final=(await result('aruvi_repositories','list_repositories',{})).find(item=>item.id===repo.id); assert.equal(final.name,'Browser updated'); assert.equal(final.local_path,paths[2]);
  assert.equal(await readFile(join(paths[2],'fixture.txt'),'utf8'),'same content\n');
  const raceArgs={id:repo.id,name:final.name,local_path:paths[3],remote_url:final.remote_url,default_branch:final.default_branch,expected:final};
  const [racedUpdate,racedRegister]=await Promise.all([call('aruvi_repositories','update_repository',raceArgs),call('aruvi_repositories','register_repository',{name:'Race registration',local_path:paths[3],default_branch:'main'})]);
  assert(!racedRegister.isError,JSON.stringify(racedRegister));
  const all=await result('aruvi_repositories','list_repositories',{}); assert.equal(all.filter(item=>item.local_path===paths[3]).length,1);
  if (!racedUpdate.isError) assert.equal(racedRegister.structuredContent.result.id,repo.id);

  const env={...process.env,ARUVI_REMOTE_URL:base,ARUVI_TEST_INSPECT:'1',ARUVI_TEST_CODE_EDITOR_PATH:paths[1],ARUVI_MOBILE_API_TOKEN:token,ARUVI_MCP_API_TOKEN:token,ARUVI_CHROME_PATH:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'};
  const suite=spawn(resolve('node_modules/.bin/playwright'),['test','-c','playwright.remote.config.js','remote.spec.js','-g','browser and MCP edit workspaces'],{stdio:'inherit',env});
  const suiteStatus=await new Promise((done,reject)=>{suite.once('error',reject);suite.once('exit',(code,signal)=>done({code,signal}));});
  if (suiteStatus.code!==0) throw new Error(`Remote browser test failed: ${suiteStatus.code ?? suiteStatus.signal}`);
  console.log('Workspace edit browser/MCP tests passed: stale snapshots, duplicates, active runs, atomic rollback and branch propagation.');
} finally { await browser?.close(); if (server && server.exitCode===null && server.signalCode===null) {server.kill(); await new Promise(r=>server.once('exit',r));} await rm(root,{recursive:true,force:true}); }
