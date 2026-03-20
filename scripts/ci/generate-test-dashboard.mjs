import fs from "node:fs";
import path from "node:path";

const workspaceRoot = process.cwd();
const reportsRoot = path.join(workspaceRoot, "reports");

function safeRead(filePath) {
  if (!fs.existsSync(filePath)) {
    return null;
  }
  return fs.readFileSync(filePath, "utf8");
}

function parseNumericAttribute(xml, attribute) {
  const match = xml.match(new RegExp(`${attribute}="(\\d+)"`));
  return match ? Number.parseInt(match[1], 10) : 0;
}

function summarizeJunit(label, relativePath) {
  const absolutePath = path.join(reportsRoot, relativePath);
  const xml = safeRead(absolutePath);
  if (!xml) {
    return {
      label,
      relativePath,
      available: false,
      tests: 0,
      failures: 0,
      skipped: 0,
      passed: 0,
    };
  }

  const tests = parseNumericAttribute(xml, "tests");
  const failures = parseNumericAttribute(xml, "failures") + parseNumericAttribute(xml, "errors");
  const skipped = parseNumericAttribute(xml, "skipped");
  return {
    label,
    relativePath,
    available: true,
    tests,
    failures,
    skipped,
    passed: Math.max(tests - failures - skipped, 0),
  };
}

function summarizePlaywrightArtifacts() {
  const artifactsDir = path.join(reportsRoot, "playwright", "artifacts");
  if (!fs.existsSync(artifactsDir)) {
    return { screenshots: 0, videos: 0, traces: 0 };
  }

  const counters = { screenshots: 0, videos: 0, traces: 0 };

  function visit(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolutePath = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        visit(absolutePath);
        continue;
      }
      if (entry.name.endsWith(".png")) {
        counters.screenshots += 1;
      } else if (entry.name.endsWith(".webm")) {
        counters.videos += 1;
      } else if (entry.name.endsWith(".zip")) {
        counters.traces += 1;
      }
    }
  }

  visit(artifactsDir);
  return counters;
}

function statusBadge(summary) {
  if (!summary.available) {
    return `<span class="badge badge-muted">missing</span>`;
  }
  if (summary.failures > 0) {
    return `<span class="badge badge-fail">failing</span>`;
  }
  return `<span class="badge badge-pass">passing</span>`;
}

function htmlEscape(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

const summaries = [
  summarizeJunit("Rust Unit Tests", path.join("rust-unit", "junit.xml")),
  summarizeJunit("Rust Integration Tests", path.join("rust-integration", "junit.xml")),
  summarizeJunit("Playwright UI Tests", path.join("playwright", "junit.xml")),
];

const artifactSummary = summarizePlaywrightArtifacts();

const markdown = [
  "## Test Reports",
  "",
  "| Suite | Status | Passed | Failed | Skipped | Total | Artifact |",
  "| --- | --- | ---: | ---: | ---: | ---: | --- |",
  ...summaries.map((summary) => {
    const status = summary.available ? (summary.failures > 0 ? "Failing" : "Passing") : "Missing";
    return `| ${summary.label} | ${status} | ${summary.passed} | ${summary.failures} | ${summary.skipped} | ${summary.tests} | \`${summary.relativePath}\` |`;
  }),
  "",
  `Playwright artifacts: ${artifactSummary.screenshots} screenshots, ${artifactSummary.videos} videos, ${artifactSummary.traces} traces`,
  "",
  "Download the `test-reports` artifact to inspect the HTML dashboard, JUnit XML, Playwright HTML report, screenshots, videos, and traces.",
  "",
].join("\n");

const html = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>AruviStudio Test Reports</title>
    <style>
      :root {
        color-scheme: dark;
        --bg: #111318;
        --panel: #1b1f27;
        --panel-border: #2f3642;
        --text: #eef3fb;
        --muted: #9aa6b7;
        --pass: #3fb68b;
        --fail: #ef6b73;
        --muted-badge: #556070;
        --link: #7db7ff;
      }
      body {
        margin: 0;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        background: radial-gradient(circle at top, #1d2330 0%, var(--bg) 50%);
        color: var(--text);
      }
      main {
        max-width: 1100px;
        margin: 0 auto;
        padding: 32px 24px 48px;
      }
      h1 {
        margin: 0 0 8px;
        font-size: 32px;
      }
      p {
        margin: 0;
        color: var(--muted);
        line-height: 1.5;
      }
      .grid {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 16px;
        margin-top: 24px;
      }
      .card,
      .artifact-card {
        background: var(--panel);
        border: 1px solid var(--panel-border);
        border-radius: 18px;
        padding: 18px;
      }
      .suite-title {
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 12px;
        margin-bottom: 12px;
      }
      .suite-title h2 {
        margin: 0;
        font-size: 18px;
      }
      .badge {
        border-radius: 999px;
        padding: 4px 10px;
        font-size: 12px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.04em;
      }
      .badge-pass {
        background: rgba(63, 182, 139, 0.18);
        color: var(--pass);
      }
      .badge-fail {
        background: rgba(239, 107, 115, 0.18);
        color: var(--fail);
      }
      .badge-muted {
        background: rgba(85, 96, 112, 0.22);
        color: #cbd5e1;
      }
      .metrics {
        display: grid;
        grid-template-columns: repeat(4, minmax(0, 1fr));
        gap: 8px;
        margin: 14px 0;
      }
      .metric {
        background: rgba(255,255,255,0.03);
        border-radius: 12px;
        padding: 12px 10px;
      }
      .metric-label {
        font-size: 11px;
        color: var(--muted);
        text-transform: uppercase;
        letter-spacing: 0.05em;
      }
      .metric-value {
        margin-top: 6px;
        font-size: 22px;
        font-weight: 800;
      }
      a {
        color: var(--link);
      }
      code {
        color: #dbe7ff;
      }
      .artifact-card {
        margin-top: 16px;
      }
      ul {
        margin: 12px 0 0;
        padding-left: 18px;
        color: var(--muted);
      }
      @media (max-width: 960px) {
        .grid {
          grid-template-columns: 1fr;
        }
      }
    </style>
  </head>
  <body>
    <main>
      <h1>AruviStudio Test Reports</h1>
      <p>Aggregated report bundle for Rust unit tests, Rust integration tests, and Playwright UI tests.</p>
      <div class="grid">
        ${summaries
          .map(
            (summary) => `
              <section class="card">
                <div class="suite-title">
                  <h2>${htmlEscape(summary.label)}</h2>
                  ${statusBadge(summary)}
                </div>
                <div class="metrics">
                  <div class="metric"><div class="metric-label">Passed</div><div class="metric-value">${summary.passed}</div></div>
                  <div class="metric"><div class="metric-label">Failed</div><div class="metric-value">${summary.failures}</div></div>
                  <div class="metric"><div class="metric-label">Skipped</div><div class="metric-value">${summary.skipped}</div></div>
                  <div class="metric"><div class="metric-label">Total</div><div class="metric-value">${summary.tests}</div></div>
                </div>
                <p>JUnit XML: <a href="${htmlEscape(summary.relativePath)}">${htmlEscape(summary.relativePath)}</a></p>
              </section>
            `,
          )
          .join("")}
      </div>
      <section class="artifact-card">
        <div class="suite-title">
          <h2>UI Debug Artifacts</h2>
          <span class="badge badge-muted">playwright</span>
        </div>
        <div class="metrics">
          <div class="metric"><div class="metric-label">Screenshots</div><div class="metric-value">${artifactSummary.screenshots}</div></div>
          <div class="metric"><div class="metric-label">Videos</div><div class="metric-value">${artifactSummary.videos}</div></div>
          <div class="metric"><div class="metric-label">Traces</div><div class="metric-value">${artifactSummary.traces}</div></div>
          <div class="metric"><div class="metric-label">HTML Report</div><div class="metric-value"><a href="playwright/html/index.html">Open</a></div></div>
        </div>
        <ul>
          <li>Playwright HTML report: <code>reports/playwright/html/index.html</code></li>
          <li>Failure screenshots, videos, and traces: <code>reports/playwright/artifacts/</code></li>
          <li>Raw JUnit XML: <code>reports/playwright/junit.xml</code></li>
        </ul>
      </section>
    </main>
  </body>
</html>`;

fs.mkdirSync(reportsRoot, { recursive: true });
fs.writeFileSync(path.join(reportsRoot, "index.html"), html);
fs.writeFileSync(path.join(reportsRoot, "summary.md"), markdown);
