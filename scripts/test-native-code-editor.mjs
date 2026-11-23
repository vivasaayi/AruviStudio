// Disposable native server and repository for the same browser editor flow used on Linux.
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn, execFileSync } from 'node:child_process';
import net from 'node:net';
const root = await mkdtemp(join(tmpdir(), 'aruvi-code-editor-'));
let server;
try {
  const repo = join(root,'repo'); await mkdir(repo); execFileSync('git',['init','-q',repo]);
  for (const [name, content] of Object.entries({'first.js':'const first = 1;\n','second.js':'const second = 2;\n','mixed.js':'one\r\ntwo\n','windows.js':'one\r\ntwo\r\n'})) await writeFile(join(repo,name),content);
  const port = await new Promise(resolvePort => { const s = net.createServer(); s.listen(0,'127.0.0.1',() => { const p=s.address().port; s.close(() => resolvePort(p)); }); });
  const token='code-editor-fixture-token', base=`http://127.0.0.1:${port}`;
  const env={...process.env, ARUVI_APP_DATA_DIR:root, ARUVI_DB_PATH:join(root,'db.sqlite'), ARUVI_WEBHOOK_HOST:'127.0.0.1', ARUVI_WEBHOOK_PORT:String(port), ARUVI_MOBILE_API_TOKEN:token, ARUVI_MCP_API_TOKEN:token, ARUVI_MONACO_ASSETS_DIR:resolve('node_modules/monaco-editor/min/vs'), ARUVI_REMOTE_URL:base, ARUVI_TEST_INSPECT:'1', ARUVI_TEST_CODE_EDITOR_PATH:repo, ARUVI_CHROME_PATH:process.env.ARUVI_CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'};
  server=spawn(resolve('src-tauri/target/debug/aruvi-studio-server'),[],{stdio:'ignore',env});
  for (let i=0;;i++) { if (server.exitCode !== null || i === 120) throw new Error('Native fixture did not start'); try { if ((await fetch(base+'/health')).ok) break; } catch {} await new Promise(r => setTimeout(r,250)); }
  const browserTest=spawn(resolve('node_modules/.bin/playwright'),['test','-c','playwright.remote.config.js','remote.spec.js','-g','optional code editor preserves tabs'],{stdio:'inherit',env});
  const status=await new Promise((resolveStatus,reject) => { browserTest.once('error',reject); browserTest.once('exit',(code,signal) => resolveStatus({code,signal})); });
  if (status.code !== 0) throw new Error(`Browser test failed: ${status.code ?? status.signal}`);
} finally {
  if (server && server.exitCode === null && server.signalCode === null) { server.kill(); await new Promise(r => server.once('exit',r)); }
  await rm(root,{recursive:true,force:true});
}
