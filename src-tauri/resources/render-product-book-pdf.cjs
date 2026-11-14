// This worker has no application secrets. It receives one JSON request on stdin
// and writes only the PDF bytes to stdout. The Rust parent limits time and size.
const { chromium } = require(process.env.ARUVI_PDF_PLAYWRIGHT_PATH);

const escapeHtml = (value) => value.replaceAll("&", "&amp;").replaceAll("<", "&lt;")
  .replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;");

async function main() {
  let input = "";
  for await (const chunk of process.stdin) input += chunk;
  const { html, name, trim } = JSON.parse(input);
  const headerTemplate = `<style>.frame { width:100%;padding:0 0.35in;font-family:Inter,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;font-size:8px;color:#6f5945;display:flex;justify-content:space-between;align-items:center;letter-spacing:0.04em;text-transform:uppercase; }</style><div class="frame"><span>${escapeHtml(name || "Aruvi Studio Book")}</span><span>${escapeHtml(trim.label)}</span></div>`;
  const footerTemplate = `<style>.frame { width:100%;padding:0 0.35in;font-family:Inter,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;font-size:8px;color:#7f6b59;display:flex;justify-content:space-between;align-items:center; }</style><div class="frame"><span>Aruvi Studio export</span><span><span class="pageNumber"></span>/<span class="totalPages"></span></span></div>`;
  const browser = await chromium.launch({ executablePath: process.env.ARUVI_PDF_BROWSER, headless: true, chromiumSandbox: process.env.ARUVI_PDF_SANDBOX !== "0" });
  const deadline = setTimeout(() => { void browser.close().catch(() => {}); }, 45000);
  try {
    const context = await browser.newContext({ javaScriptEnabled: false, serviceWorkers: "block" });
    await context.route("**/*", (route) => route.abort());
    const page = await context.newPage();
    const csp = `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:" />`;
    if (!html.includes("<head>")) throw new Error("invalid renderer output");
    await page.setContent(html.replace("<head>", "<head>" + csp), { waitUntil: "load", timeout: 30000 });
    await page.emulateMedia({ media: "print" });
    const pdf = await page.pdf({ width: trim.pageWidth, height: trim.pageHeight,
      margin: { top: trim.marginTop, right: trim.marginRight, bottom: trim.marginBottom, left: trim.marginLeft },
      printBackground: true, displayHeaderFooter: true, headerTemplate, footerTemplate,
      outline: true, tagged: true, preferCSSPageSize: false });
    await new Promise((resolve, reject) => process.stdout.write(pdf, (error) => error ? reject(error) : resolve()));
    await context.close();
  } finally {
    clearTimeout(deadline);
    await browser.close();
  }
}
main().catch(() => { process.stderr.write("PDF rendering failed\n"); process.exitCode = 1; });
