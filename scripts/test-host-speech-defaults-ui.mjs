// Exercise server speech defaults using a disposable Studio and synthetic audio.
import assert from "node:assert/strict";
import { chromium } from "@playwright/test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

let input = "";
for await (const chunk of process.stdin) input += chunk;
const { address, provider, audio } = JSON.parse(input);
const directory = mkdtempSync(join(tmpdir(), "aruvi-speech-defaults-"));
let browser;
try {
  const wav = join(directory, "fixture.wav");
  writeFileSync(wav, Buffer.from(audio, "base64"), { mode: 0o600 });
  browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true, args: ["--use-fake-device-for-media-stream", "--use-fake-ui-for-media-stream", `--use-file-for-fake-audio-capture=${wav}`] });
  const page = await browser.newPage();
  await page.goto(`http://${address}/remote`);
  await page.locator("#tokenInput").fill("workflow-mobile-test"); await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Voice", exact: true }).click();
  await page.locator(`#defaultSpeechProvider option[value="${provider}"]`).waitFor({ state: "attached" });
  await page.getByLabel("Default transcription provider").selectOption(provider);
  await page.getByLabel("Default transcription model").fill("whisper-tiny-metal");
  await page.getByLabel("Default speech locale").fill("en");
  await page.getByRole("button", { name: "Save speech defaults" }).click();
  await page.locator("#status").getByText("Speech defaults saved", { exact: true }).waitFor();
  assert.equal(await page.getByLabel("Transcription provider", { exact: true }).inputValue(), "");
  const sent = page.waitForRequest((request) => request.method() === "POST" && request.url().endsWith("/api/mobile/speech/transcribe"));
  await page.getByRole("button", { name: "Record", exact: true }).click();
  await page.getByRole("button", { name: "Stop", exact: true }).waitFor();
  await page.waitForTimeout(5000); await page.getByRole("button", { name: "Stop", exact: true }).click();
  const body = (await sent).postDataJSON();
  assert.equal(body.provider_id, undefined); assert.equal(body.model_name, undefined); assert.equal(body.locale, undefined);
  await page.locator("#voiceMeta").getByText("Transcript ready.", { exact: true }).waitFor({ timeout: 120000 });
  const text = (await page.locator("#voiceComposer").inputValue()).toLowerCase();
  assert.match(text, /\bfox\b/); assert.match(text, /\bdog\b/);
  await page.getByLabel("Transcription provider", { exact: true }).selectOption(provider);
  await page.getByLabel("Transcription model", { exact: true }).fill("whisper-tiny-metal");
  await page.getByLabel("Speech locale", { exact: true }).fill("en");
  const explicit = page.waitForRequest((request) => request.method() === "POST" && request.url().endsWith("/api/mobile/speech/transcribe"));
  await page.getByRole("button", { name: "Record", exact: true }).click();
  await page.getByRole("button", { name: "Stop", exact: true }).waitFor();
  await page.waitForTimeout(5000); await page.getByRole("button", { name: "Stop", exact: true }).click();
  const override = (await explicit).postDataJSON();
  assert.equal(override.provider_id, provider); assert.equal(override.model_name, "whisper-tiny-metal"); assert.equal(override.locale, "en");
  await page.locator("#voiceMeta").getByText("Transcript ready.", { exact: true }).waitFor({ timeout: 120000 });
  assert.match((await page.locator("#voiceComposer").inputValue()).toLowerCase(), /\bfox\b/);
  console.log("Browser saved server defaults and transcribed synthetic audio with real Model Host Whisper.");
} finally {
  if (browser) await browser.close();
  rmSync(directory, { recursive: true, force: true });
}
