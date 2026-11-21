// Launched by test-native-model-host with its disposable Studio server.
import assert from 'node:assert/strict';
import { chromium } from '@playwright/test';
const { ARUVI_FIXTURE_URL: url, ARUVI_FIXTURE_TOKEN: token, ARUVI_FIXTURE_PROVIDER: provider, ARUVI_FIXTURE_MODEL: model } = process.env;
if (!url || !token || !provider || !model) throw new Error('Missing native fixture settings');
const browser = await chromium.launch({executablePath:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
try {
  const page = await browser.newPage();
  await page.goto(url+'/remote');
  await page.locator('#tokenInput').fill(token); await page.locator('#providerInput').fill(provider); await page.locator('#modelInput').fill(model); await page.locator('#saveButton').click();
  await page.getByRole('button',{name:'Workspaces',exact:true}).click();
  await page.locator('#repositoryCopilotQuestion').fill('In one sentence, explain what a Rust ownership rule does.');
  await page.locator('#repositoryCopilotButton').click();
  await page.locator('#status').getByText('Copilot reply ready',{exact:true}).waitFor({timeout:120000});
  const transcript = await page.locator('#repositoryCopilotResult').textContent();
  assert(transcript.includes('Copilot: ') && transcript.split('Copilot: ').at(-1).trim().length > 0);
  console.log('Native browser general Copilot received a real Model Host reply without an open file.');
} finally { await browser.close(); }
