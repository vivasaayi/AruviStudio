// Isolated renderer: the only shared resource is its Unix socket. The service
// does not receive application tokens, databases, repositories, or networking.
const http = require("node:http");
const fs = require("node:fs");
const { spawn } = require("node:child_process");
const socketPath = "/pdf-socket/renderer.sock";
const worker = fs.readFileSync("/usr/local/lib/aruvi-pdf/render-product-book-pdf.cjs", "utf8");
const maxRequest = 32 * 1024 * 1024;
const maxPdf = 24 * 1024 * 1024;
let active = 0;

function validDimension(value, maxInches) {
  if (typeof value !== "string") return false;
  const match = /^(\d{1,3}(?:\.\d{1,3})?)(mm|in)$/.exec(value);
  return !!match && Number(match[1]) > 0 && Number(match[1]) / (match[2] === "mm" ? 25.4 : 1) <= maxInches;
}

const server = http.createServer((req, res) => {
  if (req.method !== "POST" || req.url !== "/render") { res.writeHead(404).end(); return; }
  if (active >= 2) { res.writeHead(503).end(); return; }
  let size = 0;
  const chunks = [];
  req.on("data", (chunk) => {
    size += chunk.length;
    if (size > maxRequest) { req.destroy(); return; }
    chunks.push(chunk);
  });
  req.on("end", () => {
    let input;
    try { input = JSON.parse(Buffer.concat(chunks).toString("utf8")); }
    catch { res.writeHead(400).end(); return; }
    const trim = input?.trim;
    if (typeof input?.html !== "string" || Buffer.byteLength(input.html) > 12 * 1024 * 1024 ||
        typeof input?.name !== "string" || input.name.length > 10000 || typeof trim?.label !== "string" || trim.label.length > 200 ||
        !validDimension(trim.pageWidth, 20) || !validDimension(trim.pageHeight, 20) ||
        ![trim.marginTop, trim.marginRight, trim.marginBottom, trim.marginLeft].every((value) => validDimension(value, 3))) {
      res.writeHead(400).end(); return;
    }
    if (active >= 2) { res.writeHead(503).end(); return; }
    active++;
    const child = spawn(process.execPath, ["--max-old-space-size=256", "-e", worker], {
      detached: true,
      env: { PATH: "/usr/bin:/bin:/usr/local/bin", HOME: "/tmp", TMPDIR: "/tmp", ARUVI_PDF_BROWSER: "/usr/bin/chromium", ARUVI_PDF_PLAYWRIGHT_PATH: "/usr/local/lib/aruvi-pdf/playwright-core", ARUVI_PDF_SANDBOX: "0" },
      stdio: ["pipe", "pipe", "ignore"],
    });
    let finished = false;
    let bytes = 0;
    const output = [];
    const terminate = () => { if (child.pid) { try { process.kill(-child.pid, "SIGTERM"); } catch {} } };
    const deadline = setTimeout(() => { terminate(); if (!res.headersSent) res.writeHead(504).end(); }, 55000);
    res.on("close", () => { if (!res.writableEnded) terminate(); });
    child.on("error", () => { if (!res.headersSent) res.writeHead(500).end(); });
    child.stdin.on("error", () => {});
    child.stdout.on("data", (chunk) => {
      bytes += chunk.length;
      if (bytes > maxPdf) { terminate(); if (!res.headersSent) res.writeHead(413).end(); return; }
      output.push(chunk);
    });
    child.on("close", (code) => {
      if (finished) return;
      finished = true; active--; clearTimeout(deadline);
      if (res.writableEnded || res.destroyed) return;
      const pdf = Buffer.concat(output);
      if (code !== 0 || !pdf.subarray(0, 5).equals(Buffer.from("%PDF-"))) { res.writeHead(500).end(); return; }
      res.writeHead(200, { "Content-Type": "application/pdf", "Content-Length": pdf.length, "Cache-Control": "no-store" }).end(pdf);
    });
    child.stdin.end(Buffer.from(JSON.stringify(input)));
  });
});
server.headersTimeout = 10000;
server.requestTimeout = 60000;
fs.rmSync(socketPath, { force: true });
server.listen(socketPath, () => fs.chmodSync(socketPath, 0o600));
