import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn, execFileSync } from 'node:child_process';
import net from 'node:net';
import { chromium } from '@playwright/test';
const root=await mkdtemp(join(tmpdir(),'aruvi-workflow-calls-'));
let server,browser;
try {
  const port=await new Promise(done=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>done(p));});});
  const base=`http://127.0.0.1:${port}`, token='workflow-calls-fixture', db=join(root,'db.sqlite');
  server=spawn(resolve('src-tauri/target/debug/aruvi-studio-server'),[],{stdio:'ignore',env:{...process.env,ARUVI_APP_DATA_DIR:root,ARUVI_DB_PATH:db,ARUVI_WEBHOOK_HOST:'127.0.0.1',ARUVI_WEBHOOK_PORT:String(port),ARUVI_MOBILE_API_TOKEN:token,ARUVI_MCP_API_TOKEN:token}});
  for(let i=0;;i++){if(server.exitCode!==null||i===120)throw Error('Server did not start');try{if((await fetch(base+'/health')).ok)break;}catch{}await new Promise(r=>setTimeout(r,250));}
  const headers={'Content-Type':'application/json',Authorization:`Bearer ${token}`};
  async function call(name,action,args){const r=await fetch(base+'/api/mcp',{method:'POST',headers,body:JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/call',params:{name,arguments:{action,arguments:args}}})});const v=(await r.json()).result;assert(!v.isError,JSON.stringify(v));return v.structuredContent.result;}
  const product=await call('aruvi_catalog','create_product',{name:'Model calls fixture'});
  const task=await call('aruvi_work_items','create_work_item',{product_id:product.id,title:'Calls fixture task',work_item_type:'task'});
  await writeFile(join(root,'artifacts','stage.txt'),'Recorded stage artifact.');
  execFileSync('python3',['-c',`import sqlite3,sys
c=sqlite3.connect(sys.argv[1])
for workflow,model in [('first','first-model'),('second','other-model')]:
 c.execute("INSERT INTO workflow_runs(id,work_item_id,status,current_stage) VALUES(?,?,?,?)",(workflow,sys.argv[2],'completed','done'))
 c.execute("INSERT INTO model_calls(id,source_kind,source_id,workflow_run_id,provider_id,model_name,token_count_input,token_count_output) VALUES(?,?,?,?,?,?,?,?)",(workflow+'-call','workflow_agent',workflow,workflow,'fixture',model,12,7))
 c.execute("INSERT INTO workflow_stage_history(id,workflow_run_id,from_stage,to_stage) VALUES(?,?,?,?)",(workflow+'-transition',workflow,workflow+'-stage','done'))
 c.execute("INSERT INTO artifacts(id,work_item_id,workflow_run_id,artifact_type,storage_path,summary) VALUES(?,?,?,?,?,?)",(workflow+'-artifact',sys.argv[2],workflow,workflow+'-stage_note',sys.argv[3],workflow+' artifact'))
c.commit()`,db,task.id,join(root,'artifacts','stage.txt')]);
  assert.equal((await fetch(base+'/api/mobile/workflows/first/model-calls')).status,401);
  const a=await call('aruvi_workflows','list_agent_model_calls_for_workflow',{workflow_run_id:'first'});
  const b=await (await fetch(base+'/api/mobile/workflows/first/model-calls',{headers})).json();
  assert.deepEqual(a.map(x=>x.id),['first-call']);assert.deepEqual(b.map(x=>x.id),['first-call']);
  assert.equal((await fetch(base+'/api/mobile/workflows/first/history')).status,401);
  const history=await call('aruvi_workflows','get_workflow_history',{workflow_run_id:'first'});
  const httpHistory=await (await fetch(base+'/api/mobile/workflows/first/history',{headers})).json();assert.deepEqual(httpHistory.map(x=>x.id),history.map(x=>x.id));assert.deepEqual(history.map(x=>x.id),['first-transition']);
  browser=await chromium.launch({executablePath:process.env.ARUVI_CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const page=await browser.newPage();await page.goto(base+'/remote');await page.locator('#tokenInput').fill(token);await page.locator('#saveButton').click();await page.getByRole('button',{name:'Work',exact:true}).click();await page.locator('#workProductSelect').selectOption(product.id);await page.getByText('Calls fixture task',{exact:false}).first().click();
  const delivery=await (await fetch(base+`/api/mobile/work-items/${task.id}/delivery`,{headers})).json();
  const expected=delivery.workflow_run.id==='first'?'first-model':'other-model';
  const unexpected=delivery.workflow_run.id==='first'?'other-model':'first-model';
  await page.getByRole('button',{name:'Review workflow stages'}).click();await page.getByLabel('Workflow stage').selectOption(delivery.workflow_run.id+'-stage');
  const stageText=await page.locator('#deliveryDetails').innerText();assert(stageText.includes(delivery.workflow_run.id+'-stage → done'));assert(!stageText.includes((delivery.workflow_run.id==='first'?'second':'first')+'-stage → done'));
  await page.getByRole('button',{name:'Show stage artifact: '+delivery.workflow_run.id+' artifact'}).click();await page.getByText('Recorded stage artifact.',{exact:true}).waitFor();assert.equal(await page.getByRole('button',{name:'Show stage artifact: '+(delivery.workflow_run.id==='first'?'second':'first')+' artifact'}).count(),0);
  await page.locator('#deliveryDetails').getByRole('button',{name:'Coding',exact:true}).click();assert.equal(await page.getByLabel('Workflow stage').inputValue(),'coding');assert((await page.locator('#deliveryDetails').innerText()).includes('No runs or transitions recorded for this stage.'));
  await page.getByRole('button',{name:'Show workflow model calls'}).click();await page.locator('#deliveryDetails pre').filter({hasText:expected}).waitFor();
  const details=await page.locator('#deliveryDetails').innerText();assert(details.includes(expected));assert(!details.includes(unexpected));
  assert(details.includes('Source: per-call telemetry'));assert(details.includes('Input tokens: 12'));assert(details.includes('Output tokens: 7'));
  execFileSync('python3',['-c',`import sqlite3,sys
c=sqlite3.connect(sys.argv[1]);c.execute("INSERT INTO agent_definitions(id,name,role) VALUES('legacy-agent','Legacy agent','developer')");c.execute("DELETE FROM model_calls WHERE workflow_run_id=?",(sys.argv[2],));c.execute("INSERT INTO agent_runs(id,workflow_run_id,agent_id,stage,status,token_count_input,token_count_output) VALUES(?,?,?,?,?,?,?)",('legacy-run',sys.argv[2],'legacy-agent','coding','completed',23,9));c.commit()`,db,delivery.workflow_run.id]);
  await page.getByRole('button',{name:'Show workflow model calls'}).click();await page.locator('#deliveryDetails pre').filter({hasText:'Source: legacy agent-run totals'}).waitFor();
  const legacy=await page.locator('#deliveryDetails').innerText();assert(legacy.includes('Input tokens: 23'));assert(legacy.includes('Output tokens: 9'));assert(legacy.includes('legacy-agent'));
  console.log('Native workflow calls, history, usage, artifact isolation and browser checks passed');
} finally {await browser?.close();if(server&&server.exitCode===null){server.kill();await new Promise(r=>server.once('exit',r));}await rm(root,{recursive:true,force:true});}
