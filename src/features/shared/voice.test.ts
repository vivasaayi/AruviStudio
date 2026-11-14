import { afterEach, describe, expect, it, vi } from "vitest";
import { startSilenceAwareWavCapture, startWavCapture } from "./voice";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

function fixture(options: { fail?: "constructor" | "resume" | "connect" | "close"; closeGate?: Promise<void>; sampleRate?: number } = {}) {
  const track = { stop: vi.fn() };
  const disconnect = vi.fn();
  const source = { connect: vi.fn(), disconnect };
  const processor = { connect: vi.fn(), disconnect, onaudioprocess: null as null | ((event: unknown) => void) };
  const sink = { connect: vi.fn(), disconnect, gain: { value: 1 } };
  if (options.fail === "connect") processor.connect.mockImplementation(() => { throw new Error("connect failed"); });
  const close = vi.fn(async () => {
    if (options.closeGate) await options.closeGate;
    if (options.fail === "close") throw new Error("close failed");
  });
  const resume = vi.fn(async () => { if (options.fail === "resume") throw new Error("resume failed"); });
  class AudioContextFixture {
    sampleRate = options.sampleRate ?? 16000;
    destination = {};
    constructor() { if (options.fail === "constructor") throw new Error("constructor failed"); }
    resume = resume;
    close = close;
    createMediaStreamSource() { return source; }
    createScriptProcessor() { return processor; }
    createGain() { return sink; }
  }
  vi.stubGlobal("window", { AudioContext: AudioContextFixture, setTimeout, clearTimeout });
  vi.stubGlobal("navigator", { mediaDevices: { getUserMedia: vi.fn(async () => ({ getTracks: () => [track] })) } });
  return { track, close, processor, disconnect };
}

describe("desktop PCM capture lifecycle", () => {
  it.each(["constructor", "resume", "connect"] as const)("releases the microphone on %s failure", async (fail) => {
    const { track, close } = fixture({ fail });
    await expect(startWavCapture()).rejects.toThrow(`${fail} failed`);
    expect(track.stop).toHaveBeenCalledOnce();
    expect(close).toHaveBeenCalledTimes(fail === "constructor" ? 0 : 1);
  });

  it("shares a single finalization between simultaneous stop and completion", async () => {
    vi.useFakeTimers();
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    const { track, close, processor } = fixture({ closeGate: gate });
    const capture = await startSilenceAwareWavCapture({ maxDurationMs: 1000 });
    processor.onaudioprocess?.({ inputBuffer: { getChannelData: () => new Float32Array([0.5, -0.5]) } });
    const first = capture.stop();
    const second = capture.stop();
    expect(close).toHaveBeenCalledOnce();
    expect(track.stop).toHaveBeenCalledOnce();
    release();
    const [a, b, completed] = await Promise.all([first, second, capture.completed]);
    expect(a).toBe(b);
    expect(a).toBe(completed);
    const wav = new DataView(await a.arrayBuffer());
    expect(wav.getUint32(24, true)).toBe(16000);
    expect(wav.getUint32(40, true)).toBe(4);
    await vi.advanceTimersByTimeAsync(1000);
    expect(close).toHaveBeenCalledOnce();
  });

  it("automatically stops manual capture at the sample limit even if timers are delayed", async () => {
    vi.useFakeTimers();
    const { track, close, processor } = fixture();
    const capture = await startWavCapture();
    processor.onaudioprocess?.({ inputBuffer: { getChannelData: () => new Float32Array(480001).fill(0.25) } });
    const blob = await capture.completed;
    expect(blob.size).toBe(44 + 480000 * 2);
    expect(new DataView(await blob.arrayBuffer()).getUint32(24, true)).toBe(16000);
    expect(track.stop).toHaveBeenCalledOnce();
    expect(close).toHaveBeenCalledOnce();
    expect(await capture.stop()).toBe(blob);
  });

  it("limits 48 kHz capture and produces a 30-second 16 kHz WAV", async () => {
    vi.useFakeTimers();
    const { track, processor } = fixture({ sampleRate: 48000 });
    const capture = await startWavCapture();
    processor.onaudioprocess?.({ inputBuffer: { getChannelData: () => new Float32Array(1440001).fill(0.25) } });
    const blob = await capture.completed;
    expect(blob.size).toBe(44 + 480000 * 2);
    expect(new DataView(await blob.arrayBuffer()).getUint32(24, true)).toBe(16000);
    expect(track.stop).toHaveBeenCalledOnce();
  });

  it("automatically stops manual capture when its time limit expires", async () => {
    vi.useFakeTimers();
    const { track, close } = fixture();
    const capture = await startWavCapture();
    await vi.advanceTimersByTimeAsync(30000);
    await expect(capture.completed).resolves.toBeInstanceOf(Blob);
    expect(track.stop).toHaveBeenCalledOnce();
    expect(close).toHaveBeenCalledOnce();
  });

  it("rejects automatic completion when closing the audio context fails", async () => {
    vi.useFakeTimers();
    const { track, close } = fixture({ fail: "close" });
    const capture = await startSilenceAwareWavCapture({ maxDurationMs: 10 });
    const rejected = expect(capture.completed).rejects.toThrow("close failed");
    await vi.advanceTimersByTimeAsync(10);
    await rejected;
    await expect(capture.stop()).rejects.toThrow("close failed");
    expect(track.stop).toHaveBeenCalledOnce();
    expect(close).toHaveBeenCalledOnce();
  });
});
