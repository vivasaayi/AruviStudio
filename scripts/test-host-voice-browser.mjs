// Exercise browser recording against real Model Host speech using synthetic audio.
import assert from "node:assert/strict";
import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const plan = JSON.parse(execFileSync("bash", ["scripts/env", "worktree", "plan"], { encoding: "utf8" }));
const env = `${plan.state}/environment.env`;
const line = readFileSync(env, "utf8").split("\n").find((value) => value.startsWith("ARUVI_MOBILE_API_TOKEN="));
if (!line) throw new Error("Mobile token is not configured");
const token = line.slice("ARUVI_MOBILE_API_TOKEN=".length);
const port = execFileSync("docker", ["compose", "--project-name", plan.project, "--project-directory", process.cwd(), "--env-file", env, "-f", "deploy/compose.worktree.yaml", "port", "app", "8787"], { encoding: "utf8" }).trim();
const providers = JSON.parse(execFileSync("scripts/mcp", ["call", "aruvi_models", '{"action":"list_providers","arguments":{}}'], { encoding: "utf8" })).result;
const provider = providers.find((item) => item.name === "Aruvi Model Host (Native Staging)");
if (!provider) throw new Error("Native Model Host provider is not configured");
const args = { action: "generate_speech", arguments: { provider_id: provider.id, model: "piper-ljspeech", text: "The quick brown fox jumps over the lazy dog." } };
const audio = JSON.parse(execFileSync("scripts/mcp", ["call", "aruvi_speech", "-"], { input: JSON.stringify(args), encoding: "utf8", maxBuffer: 24 * 1024 * 1024, timeout: 120000 })).result;
assert.equal(audio.mime_type, "audio/wav");
const directory = mkdtempSync(join(tmpdir(), "aruvi-fake-microphone-"));
let browser;
try {
  const path = join(directory, "fixture.wav");
  writeFileSync(path, Buffer.from(audio.audio_bytes_base64, "base64"), { mode: 0o600 });
  browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true,
    args: ["--use-fake-device-for-media-stream", "--use-fake-ui-for-media-stream", `--use-file-for-fake-audio-capture=${path}`] });
  const page = await browser.newPage();
  let capturedRate = null;
  page.on("request", (request) => {
    if (request.method() !== "POST" || !request.url().endsWith("/api/mobile/speech/transcribe")) return;
    const payload = request.postDataJSON();
    const wav = Buffer.from(payload.audio_bytes_base64, "base64");
    if (wav.length >= 44 && wav.toString("ascii", 0, 4) === "RIFF") capturedRate = wav.readUInt32LE(24);
  });
  await page.goto(`http://${port}/remote`);
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Voice", exact: true }).click();
  await page.locator(`#voiceProvider option[value="${provider.id}"]`).waitFor({ state: "attached" });
  await page.getByLabel("Transcription provider").selectOption(provider.id);
  await page.getByLabel("Transcription model").fill("whisper-tiny-metal");
  await page.getByLabel("Speech locale").fill("en");
  await page.getByRole("button", { name: "Record", exact: true }).click();
  await page.getByRole("button", { name: "Stop", exact: true }).waitFor();
  await page.waitForTimeout(5000);
  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await page.locator("#voiceMeta").getByText("Transcript ready.", { exact: true }).waitFor({ timeout: 120000 });
  const text = (await page.locator("#voiceComposer").inputValue()).toLowerCase();
  assert.match(text, /\bfox\b/);
  assert.match(text, /\bdog\b/);
  assert.ok(capturedRate >= 8000 && capturedRate <= 192000, "Browser sent an unsupported WAV sample rate");
  console.log(`Browser recorded synthetic Piper audio at ${capturedRate} Hz and transcribed it with real Model Host Whisper.`);
} finally {
  if (browser) await browser.close();
  rmSync(directory, { recursive: true, force: true });
}
