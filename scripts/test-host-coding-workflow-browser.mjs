// Operate only the disposable Studio fixture started by test-host-coding-workflow.
import assert from 'node:assert/strict';
import { chromium } from '@playwright/test';
const { ARUVI_FIXTURE_URL: base, ARUVI_FIXTURE_TOKEN: token, ARUVI_FIXTURE_PRODUCT: product, ARUVI_FIXTURE_TITLE: title, ARUVI_FIXTURE_GATE: gate } = process.env;
if (!base?.startsWith('http://127.0.0.1:') || !token || !product || !title || !['plan', 'test'].includes(gate)) throw new Error('Invalid isolated workflow fixture settings');
const browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless: true });
try {
  const page = await browser.newPage();
  await page.goto(base + '/remote');
  await page.locator('#tokenInput').fill(token); await page.locator('#saveButton').click();
  await page.getByRole('button', { name: 'Work', exact: true }).click();
  await page.locator('#workProductSelect').selectOption(product);
  await page.locator('#workList button.work-card').filter({hasText:title}).click();
  const status = page.locator('#deliveryStatus');
  await status.getByText(gate === 'plan' ? 'pending plan approval' : 'pending test review', { exact: true }).waitFor({timeout:30000});
  const response = page.waitForResponse(r => r.url().includes('/api/mobile/workflows/') && r.url().endsWith('/action') && r.request().method() === 'POST', {timeout:240000});
  await page.getByRole('button', {name:'Approve Gate'}).click();
  const result = await response;
  assert(result.ok(), `Workflow action returned HTTP ${result.status()}`);
  await status.getByText(gate === 'plan' ? 'pending test review' : 'done', {exact:true}).waitFor({timeout:30000});
  console.log(`Browser approved the ${gate} gate in the disposable workflow.`);
} finally { await browser.close(); }
