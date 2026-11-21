import { chromium } from "playwright";
import { readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";

const { ARUVI_REMOTE_URL: url, ARUVI_MOBILE_API_TOKEN: token, ARUVI_CHROME_PATH: executablePath, ARUVI_TEST_CI_DIGEST: digest, ARUVI_TEST_CI_PATH: artifactPath } = process.env;
if (![url, token, executablePath, digest, artifactPath].every(Boolean)) throw new Error("Missing disposable fixture configuration");
const original = await readFile(artifactPath);
const browser = await chromium.launch({ executablePath, headless: true });
try {
  const page = await browser.newPage({ acceptDownloads: true });
  await page.addInitScript(() => {
    window.__stream = { chunks: [], closed: false, aborted: false };
    window.showSaveFilePicker = async () => ({ createWritable: async () => ({
      write: async bytes => window.__stream.chunks.push(Array.from(bytes)),
      close: async () => { window.__stream.closed = true; },
      abort: async () => { window.__stream.aborted = true; },
    }) });
  });
  await page.goto(url + "/remote");
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "CI", exact: true }).click();
  await page.getByRole("button", { name: /Run #1/ }).click();
  const button = page.getByRole("button", { name: "Download artifact #1" });
  const downloading = page.waitForEvent("download");
  await button.click();
  const download = await downloading;
  const data = await readFile(await download.path());
  if (!data.equals(original) || createHash("sha256").update(data).digest("hex") !== digest || download.suggestedFilename() !== "package.bin") throw new Error("Downloaded artifact differs from the original");
  const streamButton = page.getByRole("button", { name: "Save large artifact #1" });
  await streamButton.click();
  await page.waitForFunction(() => window.__stream.closed);
  const streamed = await page.evaluate(() => window.__stream.chunks);
  if (streamed.length < 2 || !Buffer.from(streamed.flat()).equals(original)) throw new Error("Streaming artifact differs from the original");
  if (process.env.ARUVI_TEST_CI_LARGE === "1") {
    // Exercise the browser's large-file path without allocating a large Docker
    // image or retaining the synthetic output in browser memory.
    const chunk = Buffer.alloc(65536, 42), count = 4096;
    const expected = createHash("sha256");
    for (let index = 0; index < count; index++) expected.update(chunk);
    expected.update(Buffer.from([42]));
    await page.evaluate(({ hash, size }) => {
      const realFetch = window.fetch;
      window.__restoreCiFetch = () => { window.fetch = realFetch; };
      const values = Array(65536).fill(42);
      window.fetch = async (input, options) => {
        const url = String(input);
        if (url.endsWith("/data/metadata")) return new Response(JSON.stringify({size, sha256: hash}), {status: 200});
        if (url.includes("/data/chunk?offset=")) {
          const offset = Number(new URL(url, location.href).searchParams.get("offset"));
          const bytes = size - offset >= values.length ? values : [42];
          const next_offset = offset + bytes.length;
          return new Response(JSON.stringify({offset, next_offset, bytes, sha256: hash, eof: next_offset === size}), {status: 200});
        }
        return realFetch(input, options);
      };
      window.__large = {bytes: 0, writes: 0, closed: false, aborted: false};
      window.showSaveFilePicker = async () => ({createWritable: async () => ({
        write: async bytes => { window.__large.bytes += bytes.length; window.__large.writes++; },
        close: async () => { window.__large.closed = true; },
        abort: async () => { window.__large.aborted = true; },
      })});
    }, {hash: expected.digest("hex"), size: count * chunk.length + 1});
    await streamButton.click({timeout: 10000});
    await page.waitForFunction(() => window.__large.closed || window.__large.aborted, null, {timeout: 180000});
    const large = await page.evaluate(() => { window.__restoreCiFetch(); return window.__large; });
    if (!large.closed || large.aborted || large.bytes !== count * chunk.length + 1 || large.writes !== count + 1) throw new Error("Large streaming artifact was not verified and committed");
    console.log(`Synthetic large browser artifact verified: ${large.bytes} bytes in ${large.writes} writes.`);
    await page.evaluate(() => { window.showSaveFilePicker = async () => ({ createWritable: async () => ({ write: async bytes => window.__stream.chunks.push(Array.from(bytes)), close: async () => { window.__stream.closed = true; }, abort: async () => { window.__stream.aborted = true; } }) }); });
  }
  if (process.env.ARUVI_TEST_CI_REAL_LARGE_SIZE) {
    page.on("console", message => { if (message.text().startsWith("CI artifact progress:")) console.log(message.text()); });
    await page.evaluate(() => {
      window.__realLarge = {bytes: 0, writes: 0, closed: false, aborted: false, hash: window.sha256.create()};
      window.showSaveFilePicker = async () => ({createWritable: async () => ({
        write: async bytes => { window.__realLarge.hash.update(bytes); window.__realLarge.bytes += bytes.length; window.__realLarge.writes++; if (window.__realLarge.writes % 512 === 0) console.log(`CI artifact progress: ${window.__realLarge.bytes} bytes`); },
        close: async () => { window.__realLarge.closed = true; },
        abort: async () => { window.__realLarge.aborted = true; },
      })});
    });
    await page.getByRole("button", { name: "Save large artifact #2" }).click();
    try { await page.waitForFunction(() => window.__realLarge.closed || window.__realLarge.aborted, null, {timeout: 300000}); }
    catch (error) { const progress = await page.evaluate(() => ({bytes: window.__realLarge.bytes, writes: window.__realLarge.writes})); throw new Error(`Real large artifact timed out after ${progress.bytes} bytes in ${progress.writes} writes`, {cause: error}); }
    const result = await page.evaluate(() => ({bytes: window.__realLarge.bytes, writes: window.__realLarge.writes, closed: window.__realLarge.closed, aborted: window.__realLarge.aborted, digest: window.__realLarge.hash.hex()}));
    if (!result.closed || result.aborted || result.bytes !== Number(process.env.ARUVI_TEST_CI_REAL_LARGE_SIZE) || result.digest !== process.env.ARUVI_TEST_CI_REAL_LARGE_DIGEST || result.writes !== 4097) throw new Error("Real large artifact transfer failed");
    console.log(`Real CI artifact streamed through Studio: ${result.bytes} bytes in ${result.writes} writes.`);
    await page.evaluate(() => { window.showSaveFilePicker = async () => ({ createWritable: async () => ({ write: async bytes => window.__stream.chunks.push(Array.from(bytes)), close: async () => { window.__stream.closed = true; }, abort: async () => { window.__stream.aborted = true; } }) }); });
  }
  await page.evaluate(() => { window.__stream = { chunks: [], closed: false, aborted: false }; });
  const changed = Buffer.from(original); changed[0] ^= 1;
  await writeFile(artifactPath, changed);
  await streamButton.click();
  await page.waitForFunction(() => window.__stream.aborted);
  if (await page.evaluate(() => window.__stream.closed)) throw new Error("Changed streaming artifact was committed");
  await button.click();
  await page.getByText("Artifact download failed: Artifact digest mismatch").waitFor();
  console.log("Browser artifact download verified; changed bytes were rejected.");
} finally {
  await writeFile(artifactPath, original);
  await browser.close();
}
