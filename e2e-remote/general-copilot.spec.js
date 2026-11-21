import { test, expect } from '@playwright/test';
import { readFile } from 'node:fs/promises';

test('general Copilot keeps bounded conversation without a selected file', async ({ page }) => {
  const html = await readFile('src-tauri/src/services/remote_app.html', 'utf8');
  await page.route('**/remote', route => route.fulfill({ status: 200, contentType: 'text/html', body: html }));
  const requests = [];
  await page.route('**/api/mobile/ide/copilot**', async route => {
    requests.push(route.request().postDataJSON());
    if (route.request().url().endsWith('/stream')) {
      await route.fulfill({ status: 200, contentType: 'application/x-ndjson', body: '{"type":"delta","content":"Streamed reply"}\n{"type":"done"}\n' });
    } else await route.fulfill({ status: 200, json: { content: 'First reply' } });
  });
  await page.goto('/remote');
  await page.locator('#providerInput').fill('provider');
  await page.locator('#modelInput').fill('model');
  await page.locator('#tokenInput').fill('test-token');
  await page.getByRole('button', { name: 'Workspaces', exact: true }).click();
  await expect(page.locator('#repositoryCopilotButton')).toBeEnabled();
  await expect(page.locator('#repositoryCopilotPatchButton')).toBeDisabled();
  await page.locator('#repositoryCopilotQuestion').fill('Explain Rust ownership');
  await page.locator('#repositoryCopilotButton').click();
  await expect(page.locator('#repositoryCopilotResult')).toContainText('First reply');
  expect(requests[0].messages.at(-1)).toEqual({ role: 'user', content: 'Explain Rust ownership' });
  expect(requests[0]).not.toHaveProperty('repository_id');
  await page.locator('#repositoryCopilotStream').check();
  await page.locator('#repositoryCopilotQuestion').fill('Give an example');
  await page.locator('#repositoryCopilotButton').click();
  await expect(page.locator('#repositoryCopilotResult')).toContainText('Streamed reply');
  expect(requests[1].messages.slice(-3)).toEqual([{role:'user', content:'Explain Rust ownership'}, {role:'assistant', content:'First reply'}, {role:'user', content:'Give an example'}]);
  await page.locator('#repositoryCopilotReset').click();
  await expect(page.locator('#repositoryCopilotResult')).toBeEmpty();
  await page.locator('#repositoryCopilotQuestion').fill('New question');
  await page.locator('#repositoryCopilotButton').click();
  await expect(page.locator('#repositoryCopilotResult')).toContainText('Streamed reply');
  expect(requests[2].messages).toHaveLength(2);
});
