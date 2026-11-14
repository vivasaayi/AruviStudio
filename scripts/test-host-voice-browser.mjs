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
const plannerMode = process.argv.includes("--planner");
const provider = providers.find((item) => item.name === "Aruvi Model Host (Native Staging)");
if (!provider) throw new Error("Native Model Host provider is not configured");
const args = { action: "generate_speech", arguments: { provider_id: provider.id, model: "piper-ljspeech", text: plannerMode ? "Show draft." : "The quick brown fox jumps over the lazy dog." } };
const audio = JSON.parse(execFileSync("scripts/mcp", ["call", "aruvi_speech", "-"], { input: JSON.stringify(args), encoding: "utf8", maxBuffer: 24 * 1024 * 1024, timeout: 120000 })).result;
assert.equal(audio.mime_type, "audio/wav");
const directory = mkdtempSync(join(tmpdir(), "aruvi-fake-microphone-"));
let browser;
try {
  const path = join(directory, "fixture.wav");
  let wav = Buffer.from(audio.audio_bytes_base64, "base64");
  if (plannerMode) {
    // Chrome loops fake microphone files. Pad this short command with silence
    // so the five-second recording contains the command only once.
    wav = execFileSync("python3", ["-c", `import io, sys, wave
source = wave.open(io.BytesIO(sys.stdin.buffer.read()), 'rb')
channels, width, rate, frames = source.getnchannels(), source.getsampwidth(), source.getframerate(), source.getnframes()
if width != 2: raise ValueError('Expected 16-bit PCM fixture')
samples = source.readframes(frames)
samples += bytes(max(0, rate * 7 - frames) * channels * width)
out = io.BytesIO()
with wave.open(out, 'wb') as writer:
    writer.setnchannels(channels); writer.setsampwidth(width); writer.setframerate(rate); writer.writeframes(samples)
sys.stdout.buffer.write(out.getvalue())`], { input: wav, maxBuffer: 4 * 1024 * 1024 });
  }
  writeFileSync(path, wav, { mode: 0o600 });
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
  if (plannerMode) {
    const coding = providers.find((item) => item.name === "Aruvi Model Host (Studio Coding Validation)");
    if (!coding) throw new Error("Coding validation provider is not configured");
    await page.locator("#providerInput").fill(coding.id);
    await page.locator("#modelInput").fill(process.env.ARUVI_CODING_TEST_MODEL || "qwen25-coder-7b-q4");
  }
  await page.locator("#saveButton").click();
  if (plannerMode) {
    await page.getByRole("button", { name: "Planner", exact: true }).click();
    await page.waitForFunction(() => !document.querySelector("#plannerProductSelect").disabled);
    await page.locator("#plannerProductSelect").selectOption("");
  }
  await page.getByRole("button", { name: "Voice", exact: true }).click();
  if (plannerMode) await page.getByLabel("Send transcript to").selectOption("planner");
  await page.locator(`#voiceProvider option[value="${provider.id}"]`).waitFor({ state: "attached" });
  await page.getByLabel("Transcription provider", { exact: true }).selectOption(provider.id);
  await page.getByLabel("Transcription model", { exact: true }).fill("whisper-tiny-metal");
  await page.getByLabel("Speech locale", { exact: true }).fill("en");
  await page.getByRole("button", { name: "Record", exact: true }).click();
  await page.getByRole("button", { name: "Stop", exact: true }).waitFor();
  await page.waitForTimeout(5000);
  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await page.locator("#voiceMeta").getByText("Transcript ready.", { exact: true }).waitFor({ timeout: 120000 });
  const text = (await page.locator("#voiceComposer").inputValue()).toLowerCase();
  if (plannerMode) {
    assert.ok(text.trim(), "Expected a nonempty real transcription");
    // The review step allows correcting recognition before invoking a command.
    await page.locator("#voiceComposer").fill("show draft");
    const submitted = page.waitForResponse((response) => response.url().endsWith("/voice-turn"), { timeout: 120000 });
    await page.locator("#voiceSendButton").click();
    const response = await submitted;
    assert.equal(response.status(), 200);
    assert.equal(response.request().postDataJSON().product_id, null);
    await page.locator("#voiceConversation").getByText("There is no staged design tree yet.").waitFor({ timeout: 120000 });
  } else {
    assert.match(text, /\bfox\b/);
    assert.match(text, /\bdog\b/);
  }
  assert.ok(capturedRate >= 8000 && capturedRate <= 192000, "Browser sent an unsupported WAV sample rate");
  console.log(`Browser recorded synthetic Piper audio at ${capturedRate} Hz and transcribed it with real Model Host Whisper${plannerMode ? "; a reviewed and corrected transcript reached the planner voice endpoint" : ""}.`);
} finally {
  if (browser) await browser.close();
  rmSync(directory, { recursive: true, force: true });
}
