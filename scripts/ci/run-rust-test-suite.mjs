import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const workspaceRoot = process.cwd();
const suite = process.argv[2];

if (suite !== "unit" && suite !== "integration") {
  console.error('Usage: node scripts/ci/run-rust-test-suite.mjs <unit|integration>');
  process.exit(1);
}

const reportsRoot = path.join(workspaceRoot, "reports");
const suiteDirectoryName = suite === "unit" ? "rust-unit" : "rust-integration";
const suiteLabel = suite === "unit" ? "Rust Unit Tests" : "Rust Integration Tests";
const reportDir = path.join(reportsRoot, suiteDirectoryName);
const rawOutputPath = path.join(reportDir, "output.txt");
const junitPath = path.join(reportDir, "junit.xml");
const htmlPath = path.join(reportDir, "index.html");

fs.mkdirSync(reportDir, { recursive: true });

function escapeXml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function parseTests(output) {
  const tests = [];
  const lines = output.split(/\r?\n/);
  const regex = /^test\s+(.+?)\s+\.\.\.\s+(ok|FAILED|ignored)$/;

  for (const line of lines) {
    const match = line.match(regex);
    if (!match) {
      continue;
    }
    tests.push({
      name: match[1],
      status: match[2] === "ok" ? "passed" : match[2] === "FAILED" ? "failed" : "skipped",
    });
  }

  return tests;
}

function buildJunitXml(label, tests) {
  const failures = tests.filter((entry) => entry.status === "failed").length;
  const skipped = tests.filter((entry) => entry.status === "skipped").length;
  const cases = tests
    .map((entry) => {
      const parts = entry.name.split("::");
      const testName = parts.pop() ?? entry.name;
      const className = parts.join("::") || label;
      if (entry.status === "failed") {
        return `    <testcase classname="${escapeXml(className)}" name="${escapeXml(testName)}"><failure message="Test failed"/></testcase>`;
      }
      if (entry.status === "skipped") {
        return `    <testcase classname="${escapeXml(className)}" name="${escapeXml(testName)}"><skipped/></testcase>`;
      }
      return `    <testcase classname="${escapeXml(className)}" name="${escapeXml(testName)}"/>`;
    })
    .join("\n");

  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    `<testsuites tests="${tests.length}" failures="${failures}" errors="0" skipped="${skipped}">`,
    `  <testsuite name="${escapeXml(label)}" tests="${tests.length}" failures="${failures}" errors="0" skipped="${skipped}">`,
    cases,
    "  </testsuite>",
    "</testsuites>",
    "",
  ].join("\n");
}

function buildHtmlReport(label, tests, rawOutput, note) {
  const failures = tests.filter((entry) => entry.status === "failed").length;
  const skipped = tests.filter((entry) => entry.status === "skipped").length;
  const passed = tests.filter((entry) => entry.status === "passed").length;

  const rows = tests.length
    ? tests
        .map((entry) => {
          const badgeClass = entry.status === "passed" ? "pass" : entry.status === "failed" ? "fail" : "skip";
          return `<tr><td>${escapeHtml(entry.name)}</td><td><span class="badge ${badgeClass}">${escapeHtml(entry.status)}</span></td></tr>`;
        })
        .join("")
    : `<tr><td colspan="2">${escapeHtml(note ?? "No tests were recorded for this suite.")}</td></tr>`;

  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>${escapeHtml(label)}</title>
    <style>
      body { margin: 0; font-family: Inter, ui-sans-serif, system-ui, sans-serif; background: #111318; color: #eef3fb; }
      main { max-width: 1100px; margin: 0 auto; padding: 32px 24px 48px; }
      h1 { margin: 0 0 8px; font-size: 30px; }
      p { color: #9aa6b7; }
      .summary { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; margin: 24px 0; }
      .card { background: #1b1f27; border: 1px solid #2f3642; border-radius: 16px; padding: 16px; }
      .label { font-size: 11px; text-transform: uppercase; color: #9aa6b7; letter-spacing: 0.05em; }
      .value { margin-top: 6px; font-size: 24px; font-weight: 800; }
      table { width: 100%; border-collapse: collapse; background: #1b1f27; border: 1px solid #2f3642; border-radius: 16px; overflow: hidden; }
      th, td { padding: 12px 14px; text-align: left; border-bottom: 1px solid #2a303b; }
      th { font-size: 12px; text-transform: uppercase; color: #9aa6b7; letter-spacing: 0.05em; }
      pre { background: #1b1f27; border: 1px solid #2f3642; border-radius: 16px; padding: 16px; overflow: auto; white-space: pre-wrap; color: #d9e2f2; }
      .badge { border-radius: 999px; padding: 3px 10px; font-size: 12px; font-weight: 700; text-transform: uppercase; }
      .pass { background: rgba(63,182,139,0.18); color: #3fb68b; }
      .fail { background: rgba(239,107,115,0.18); color: #ef6b73; }
      .skip { background: rgba(214,184,104,0.18); color: #d6b868; }
    </style>
  </head>
  <body>
    <main>
      <h1>${escapeHtml(label)}</h1>
      <p>${escapeHtml(note ?? "Generated locally and in CI by the same npm entrypoint.")}</p>
      <div class="summary">
        <div class="card"><div class="label">Passed</div><div class="value">${passed}</div></div>
        <div class="card"><div class="label">Failed</div><div class="value">${failures}</div></div>
        <div class="card"><div class="label">Skipped</div><div class="value">${skipped}</div></div>
        <div class="card"><div class="label">Total</div><div class="value">${tests.length}</div></div>
      </div>
      <table>
        <thead><tr><th>Test</th><th>Status</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
      <h2 style="margin-top: 28px;">Raw Output</h2>
      <pre>${escapeHtml(rawOutput)}</pre>
    </main>
  </body>
</html>`;
}

function writeOutputs(tests, rawOutput, note) {
  fs.writeFileSync(rawOutputPath, rawOutput);
  fs.writeFileSync(junitPath, buildJunitXml(suiteLabel, tests));
  fs.writeFileSync(htmlPath, buildHtmlReport(suiteLabel, tests, rawOutput, note));
}

function generateDashboard() {
  const dashboardScript = path.join(workspaceRoot, "scripts", "ci", "generate-test-dashboard.mjs");
  spawnSync("node", [dashboardScript], { cwd: workspaceRoot, stdio: "inherit" });
}

if (suite === "integration") {
  const integrationDir = path.join(workspaceRoot, "src-tauri", "tests");
  const hasIntegrationTests =
    fs.existsSync(integrationDir) &&
    fs.readdirSync(integrationDir).some((entry) => entry.endsWith(".rs"));

  if (!hasIntegrationTests) {
    const note = "No Rust integration tests found in src-tauri/tests.";
    writeOutputs([], `${note}\n`, note);
    generateDashboard();
    console.log(note);
    process.exit(0);
  }
}

const cargoArgs = suite === "unit"
  ? ["test", "--manifest-path", "src-tauri/Cargo.toml", "--lib"]
  : ["test", "--manifest-path", "src-tauri/Cargo.toml", "--tests"];

const result = spawnSync("cargo", cargoArgs, {
  cwd: workspaceRoot,
  encoding: "utf8",
});

const combinedOutput = [result.stdout ?? "", result.stderr ?? ""].filter(Boolean).join("\n");
const parsedTests = parseTests(combinedOutput);

writeOutputs(parsedTests, combinedOutput, null);
generateDashboard();

if (result.stdout) {
  process.stdout.write(result.stdout);
}
if (result.stderr) {
  process.stderr.write(result.stderr);
}

process.exit(result.status ?? 1);
