import { pathToFileURL } from "node:url";
import { chromium } from "playwright";

const [
  inputPath,
  outputPath,
  pageWidth,
  pageHeight,
  marginTop,
  marginRight,
  marginBottom,
  marginLeft,
  headerTitle,
  headerRight,
] = process.argv.slice(2);

if (!inputPath || !outputPath) {
  console.error("Usage: node scripts/export-book-pdf.mjs <input> <output> <width> <height> <margins...> <headerTitle> <headerRight>");
  process.exit(1);
}

const escapeHtml = (value) => value
  .replaceAll("&", "&amp;")
  .replaceAll("<", "&lt;")
  .replaceAll(">", "&gt;")
  .replaceAll("\"", "&quot;")
  .replaceAll("'", "&#39;");

const headerTemplate = `
  <style>
    .frame {
      width: 100%;
      padding: 0 0.35in;
      font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      font-size: 8px;
      color: #6f5945;
      display: flex;
      justify-content: space-between;
      align-items: center;
      letter-spacing: 0.04em;
      text-transform: uppercase;
    }
  </style>
  <div class="frame">
    <span>${escapeHtml(headerTitle || "Aruvi Studio Book")}</span>
    <span>${escapeHtml(headerRight || "")}</span>
  </div>
`;

const footerTemplate = `
  <style>
    .frame {
      width: 100%;
      padding: 0 0.35in;
      font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      font-size: 8px;
      color: #7f6b59;
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
  </style>
  <div class="frame">
    <span>Aruvi Studio export</span>
    <span class="pageNumber"></span>/<span class="totalPages"></span>
  </div>
`;

const browser = await chromium.launch({ headless: true });

try {
  const page = await browser.newPage();
  await page.goto(pathToFileURL(inputPath).href, { waitUntil: "networkidle" });
  await page.emulateMedia({ media: "print" });
  await page.pdf({
    path: outputPath,
    width: pageWidth,
    height: pageHeight,
    margin: {
      top: marginTop,
      right: marginRight,
      bottom: marginBottom,
      left: marginLeft,
    },
    printBackground: true,
    displayHeaderFooter: true,
    headerTemplate,
    footerTemplate,
    outline: true,
    tagged: true,
    preferCSSPageSize: false,
  });
} finally {
  await browser.close();
}
