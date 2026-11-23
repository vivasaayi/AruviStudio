// Run selected remote browser tests against a disposable native Studio server.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn } from 'node:child_process';
import net from 'node:net';

if (process.argv.length <= 2) throw Error('Pass Playwright arguments, for example: -g "browser places work in hierarchy"');
const root = await mkdtemp(join(tmpdir(), 'aruvi-native-remote-'));
let server;
try {
  const port = await new Promise(done => { const socket = net.createServer(); socket.listen(0, '127.0.0.1', () => { const value = socket.address().port; socket.close(() => done(value)); }); });
  const token = 'native-remote-fixture', base = `http://127.0.0.1:${port}`;
  const env = { ...process.env, ARUVI_APP_DATA_DIR:root, ARUVI_DB_PATH:join(root, 'db.sqlite'), ARUVI_WEBHOOK_HOST:'127.0.0.1', ARUVI_WEBHOOK_PORT:String(port), ARUVI_MOBILE_API_TOKEN:token, ARUVI_MCP_API_TOKEN:token, ARUVI_REMOTE_URL:base, ARUVI_CHROME_PATH:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' };
  server = spawn(resolve('src-tauri/target/debug/aruvi-studio-server'), [], { stdio:'ignore', env });
  for (let i = 0;; i++) { if (server.exitCode !== null || i === 120) throw Error('Native fixture did not start'); try { if ((await fetch(base + '/health')).ok) break; } catch {} await new Promise(done => setTimeout(done, 250)); }
  const browser = spawn(resolve('node_modules/.bin/playwright'), ['test', '-c', 'playwright.remote.config.js', 'remote.spec.js', ...process.argv.slice(2), '--workers=1'], { stdio:'inherit', env });
  const result = await new Promise((done, reject) => { browser.once('error', reject); browser.once('exit', (code, signal) => done({ code, signal })); });
  if (result.code !== 0) throw Error(`Native remote browser test failed: ${result.code ?? result.signal}`);
} finally {
  if (server && server.exitCode === null && server.signalCode === null) { server.kill(); await new Promise(done => server.once('exit', done)); }
  await rm(root, { recursive:true, force:true });
}
