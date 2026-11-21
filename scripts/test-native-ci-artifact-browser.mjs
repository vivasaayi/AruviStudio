import { chromium } from "playwright";
import { readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";

const { ARUVI_REMOTE_URL: url, ARUVI_MOBILE_API_TOKEN: token, ARUVI_CHROME_PATH: executablePath, ARUVI_TEST_CI_DIGEST: digest, ARUVI_TEST_CI_PATH: artifactPath } = process.env;
if (![url, token, executablePath, digest, artifactPath].every(Boolean)) throw new Error("Missing disposable fixture configuration");
const original = await readFile(artifactPath);
const browser = await chromium.launch({ executablePath, headless: true });
try {
  const page = await browser.newPage({ acceptDownloads: true });
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
  const changed = Buffer.from(original); changed[0] ^= 1;
  await writeFile(artifactPath, changed);
  await button.click();
  await page.getByText("Artifact download failed: Artifact digest mismatch").waitFor();
  console.log("Browser artifact download verified; changed bytes were rejected.");
} finally {
  await writeFile(artifactPath, original);
  await browser.close();
}
