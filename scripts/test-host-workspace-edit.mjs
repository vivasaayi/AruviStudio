// Validate deployed branch workspace editing using a unique unattached record.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { chromium } from '@playwright/test';
const plan=JSON.parse(execFileSync('bash',['scripts/env','worktree','plan'],{encoding:'utf8'}));
const envFile=`${plan.state}/environment.env`;
const line=readFileSync(envFile,'utf8').split('\n').find(value=>value.startsWith('ARUVI_MOBILE_API_TOKEN='));
if (!line) throw new Error('Mobile token missing');
const token=line.slice('ARUVI_MOBILE_API_TOKEN='.length);
const address=execFileSync('docker',['compose','--project-name',plan.project,'--project-directory',process.cwd(),'--env-file',envFile,'-f','deploy/compose.worktree.yaml','port','app','8787'],{encoding:'utf8'}).trim();
if (!address.startsWith('127.0.0.1:')) throw new Error('Expected loopback worktree endpoint');
const suffix=randomUUID(), name='Workspace acceptance '+suffix, path='/tmp/aruvi-workspace-acceptance-'+suffix;
function call(action,args) {
  const run=spawnSync('scripts/mcp',['call','aruvi_repositories','-'],{input:JSON.stringify({action,arguments:args}),encoding:'utf8'});
  if (run.error) throw run.error;
  if (run.status!==0) throw new Error(run.stderr.trim() || 'MCP call failed');
  return JSON.parse(run.stdout).result;
}
let browser,repository,owned=false;
try {
  repository=call('register_repository',{name,local_path:path,default_branch:'main'});
  assert.equal(repository.name,name); assert.equal(repository.local_path,path); owned=true;
  browser=await chromium.launch({executablePath:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const page=await browser.newPage(); await page.goto(`http://${address}/remote`); await page.locator('#tokenInput').fill(token); await page.locator('#saveButton').click();
  await page.getByRole('button',{name:'Workspaces',exact:true}).click();
  const card=page.locator('#repositoryList .tool-card').filter({hasText:path}); await card.getByText('Edit workspace',{exact:true}).click();
  await card.getByLabel('Edit display name').fill(name+' updated'); await card.getByLabel('Default branch').fill('acceptance-branch');
  const pending=page.waitForResponse(response=>response.url().endsWith('/update') && response.request().method()==='POST');
  await card.getByRole('button',{name:'Save workspace'}).click(); const response=await pending; assert.equal(response.status(),200,await response.text());
  const current=call('list_repositories',{}).find(item=>item.id===repository.id); assert.equal(current.name,name+' updated'); assert.equal(current.default_branch,'acceptance-branch');
  const stale={id:repository.id,name,local_path:path,remote_url:'',default_branch:'main',expected:repository};
  assert.throws(()=>call('update_repository',stale));
  console.log('Branch browser workspace edit passed; MCP verified the update and rejected a stale snapshot.');
} finally {
  await browser?.close();
  if (owned) { call('delete_repository',{id:repository.id}); console.log('Deleted the unattached workspace fixture record.'); }
}
