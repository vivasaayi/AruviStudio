// Create an identifiable fixture through the deployed branch browser, verify it
// through MCP, and archive only that fixture product on completion.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { chromium } from '@playwright/test';
const plan=JSON.parse(execFileSync('bash',['scripts/env','worktree','plan'],{encoding:'utf8'}));
const envFile=`${plan.state}/environment.env`;
const line=readFileSync(envFile,'utf8').split('\n').find(value => value.startsWith('ARUVI_MOBILE_API_TOKEN='));
if (!line) throw new Error('Mobile token missing');
const token=line.slice('ARUVI_MOBILE_API_TOKEN='.length);
const address=execFileSync('docker',['compose','--project-name',plan.project,'--project-directory',process.cwd(),'--env-file',envFile,'-f','deploy/compose.worktree.yaml','port','app','8787'],{encoding:'utf8'}).trim();
if (!address.startsWith('127.0.0.1:')) throw new Error('Expected loopback worktree endpoint');
const id='bulk-import-fixture-'+randomUUID(), name='Bulk import acceptance '+id.slice(-8);
function call(action,args) { return JSON.parse(execFileSync('scripts/mcp',['call','aruvi_catalog','-'],{input:JSON.stringify({action,arguments:args}),encoding:'utf8'})).result; }
let browser,submitted=false,jobId;
try {
  browser=await chromium.launch({executablePath:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const page=await browser.newPage(); await page.goto(`http://${address}/remote`); await page.locator('#tokenInput').fill(token); await page.locator('#saveButton').click();
  await page.getByRole('button',{name:'Products',exact:true}).click(); await page.getByText('Import JSON or CSV',{exact:true}).click();
  const content=JSON.stringify({product:{id,name},productAreas:[{id:id+'-area',name:'Acceptance area'}]});
  await page.locator('#bulkImportFile').setInputFiles({name:'acceptance.json',mimeType:'application/json',buffer:Buffer.from(content)});
  page.once('dialog',dialog => dialog.accept());
  const pending=page.waitForResponse(response => response.url().endsWith('/api/mobile/bulk-import') && response.request().method()==='POST');
  await page.getByRole('button',{name:'Start import'}).click(); const response=await pending; assert.equal(response.status(),202); submitted=true; const job=await response.json(); jobId=job.id;
  let result; for (let attempt=0;attempt<100;attempt++) { result=call('get_bulk_import_status',{job_id:job.id}); if (['completed','failed'].includes(result.job.status)) break; await new Promise(r=>setTimeout(r,100)); }
  assert.equal(result?.job?.status,'completed',JSON.stringify(result));
  const product=call('get_product',{id}); assert.equal(product.name,name);
  console.log('Branch browser import completed and the created product was verified through MCP.');
} finally {
  await browser?.close();
  if (submitted) {
    try {
      if (!jobId) throw new Error("Import job ID unavailable; inspect the fixture before cleanup");
      let state;
      for (let attempt=0;attempt<120;attempt++) {
        state=call('get_bulk_import_status',{job_id:jobId});
        if (['completed','failed'].includes(state.job.status)) break;
        await new Promise(r=>setTimeout(r,500));
      }
      if (!['completed','failed'].includes(state?.job?.status)) throw new Error("Import remains active; fixture cleanup deferred");
      const products=call('list_products',{});
      if (products.some(product=>product.id===id)) { call('archive_product',{id}); console.log('Archived the import fixture product.'); }
    } catch (error) { console.error('Unable to archive import fixture:',id); throw error; }
  }
}
