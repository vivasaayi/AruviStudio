export type ActiveAudioCapture = {
  stream: MediaStream;
  stop: () => Promise<Blob>;
};

export type SilenceAwareAudioCapture = ActiveAudioCapture & {
  completed: Promise<Blob>;
};

export function speakInBrowser(text: string) {
  if (typeof window === "undefined" || !("speechSynthesis" in window)) {
    return;
  }
  window.speechSynthesis.cancel();
  const utterance = new SpeechSynthesisUtterance(text);
  utterance.rate = 1;
  utterance.pitch = 1;
  window.speechSynthesis.speak(utterance);
}

export function speakInBrowserAsync(text: string) {
  if (typeof window === "undefined" || !("speechSynthesis" in window)) {
    return Promise.resolve();
  }
  return new Promise<void>((resolve) => {
    window.speechSynthesis.cancel();
    const utterance = new SpeechSynthesisUtterance(text);
    utterance.rate = 1;
    utterance.pitch = 1;
    utterance.onend = () => resolve();
    utterance.onerror = () => resolve();
    window.speechSynthesis.speak(utterance);
  });
}

export async function blobToBase64(blob: Blob) {
  const buffer = await blob.arrayBuffer();
  let binary = "";
  const bytes = new Uint8Array(buffer);
  for (let index = 0; index < bytes.byteLength; index += 1) {
    binary += String.fromCharCode(bytes[index]);
  }
  return window.btoa(binary);
}

function mergeFloat32Chunks(chunks: Float32Array[]) {
  const totalLength = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const merged = new Float32Array(totalLength);
  let offset = 0;
  for (const chunk of chunks) {
    merged.set(chunk, offset);
    offset += chunk.length;
  }
  return merged;
}

function downsampleAudioBuffer(samples: Float32Array, sourceRate: number, targetRate: number) {
  if (sourceRate === targetRate || samples.length === 0) {
    return samples;
  }
  const ratio = sourceRate / targetRate;
  const targetLength = Math.max(1, Math.round(samples.length / ratio));
  const output = new Float32Array(targetLength);
  let offsetResult = 0;
  let offsetBuffer = 0;
  while (offsetResult < output.length) {
    const nextOffsetBuffer = Math.min(
      samples.length,
      Math.round((offsetResult + 1) * ratio),
    );
    let accumulator = 0;
    let count = 0;
    for (let index = offsetBuffer; index < nextOffsetBuffer; index += 1) {
      accumulator += samples[index];
      count += 1;
    }
    output[offsetResult] = count > 0 ? accumulator / count : samples[offsetBuffer] ?? 0;
    offsetResult += 1;
    offsetBuffer = nextOffsetBuffer;
  }
  return output;
}

function encodePcm16Wav(samples: Float32Array, sampleRate: number) {
  const bytesPerSample = 2;
  const dataLength = samples.length * bytesPerSample;
  const buffer = new ArrayBuffer(44 + dataLength);
  const view = new DataView(buffer);

  const writeString = (offset: number, value: string) => {
    for (let index = 0; index < value.length; index += 1) {
      view.setUint8(offset + index, value.charCodeAt(index));
    }
  };

  writeString(0, "RIFF");
  view.setUint32(4, 36 + dataLength, true);
  writeString(8, "WAVE");
  writeString(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * bytesPerSample, true);
  view.setUint16(32, bytesPerSample, true);
  view.setUint16(34, 16, true);
  writeString(36, "data");
  view.setUint32(40, dataLength, true);

  let offset = 44;
  for (let index = 0; index < samples.length; index += 1) {
    const sample = Math.max(-1, Math.min(1, samples[index]));
    view.setInt16(offset, sample < 0 ? sample * 0x8000 : sample * 0x7fff, true);
    offset += bytesPerSample;
  }

  return new Blob([buffer], { type: "audio/wav" });
}

type PcmCaptureOptions = {
  silenceDurationMs?: number;
  minSpeechDurationMs?: number;
  maxDurationMs?: number;
  silenceThreshold?: number;
};

async function createPcmCapture(options?: PcmCaptureOptions) {
  const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
  const audioContextCtor = window.AudioContext
    || (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;

  if (!audioContextCtor) {
    stream.getTracks().forEach((track) => track.stop());
    throw new Error("Audio capture is not supported in this runtime.");
  }

  const audioContext = new audioContextCtor();
  await audioContext.resume();
  const sampleRate = audioContext.sampleRate;
  const source = audioContext.createMediaStreamSource(stream);
  const processor = audioContext.createScriptProcessor(4096, 1, 1);
  const sink = audioContext.createGain();
  sink.gain.value = 0;
  const chunks: Float32Array[] = [];
  const startedAt = Date.now();
  const silenceDurationMs = options?.silenceDurationMs ?? null;
  const minSpeechDurationMs = options?.minSpeechDurationMs ?? 500;
  const maxDurationMs = options?.maxDurationMs ?? null;
  const silenceThreshold = options?.silenceThreshold ?? 0.012;
  let heardSpeech = false;
  let lastSpeechAt = Date.now();
  let finished = false;
  let maxTimer: number | null = null;
  let resolveCompleted: ((blob: Blob) => void) | null = null;
  let rejectCompleted: ((reason?: unknown) => void) | null = null;

  const completed = new Promise<Blob>((resolve, reject) => {
    resolveCompleted = resolve;
    rejectCompleted = reject;
  });

  const finalize = async () => {
    if (finished) {
      const merged = mergeFloat32Chunks(chunks);
      const resampled = downsampleAudioBuffer(merged, sampleRate, 16_000);
      return encodePcm16Wav(resampled, 16_000);
    }
    finished = true;
    if (maxTimer !== null) {
      window.clearTimeout(maxTimer);
    }
    processor.disconnect();
    source.disconnect();
    sink.disconnect();
    stream.getTracks().forEach((track) => track.stop());
    await audioContext.close();
    const merged = mergeFloat32Chunks(chunks);
    const resampled = downsampleAudioBuffer(merged, sampleRate, 16_000);
    const blob = encodePcm16Wav(resampled, 16_000);
    resolveCompleted?.(blob);
    return blob;
  };

  processor.onaudioprocess = (event) => {
    if (finished) {
      return;
    }
    const input = event.inputBuffer.getChannelData(0);
    chunks.push(new Float32Array(input));
    if (silenceDurationMs == null) {
      return;
    }
    let sumSquares = 0;
    for (let index = 0; index < input.length; index += 1) {
      sumSquares += input[index] * input[index];
    }
    const rms = Math.sqrt(sumSquares / Math.max(1, input.length));
    const now = Date.now();
    if (rms >= silenceThreshold) {
      heardSpeech = true;
      lastSpeechAt = now;
      return;
    }
    if (heardSpeech && now - startedAt >= minSpeechDurationMs && now - lastSpeechAt >= silenceDurationMs) {
      void finalize();
    }
  };

  source.connect(processor);
  processor.connect(sink);
  sink.connect(audioContext.destination);

  if (maxDurationMs != null) {
    maxTimer = window.setTimeout(() => {
      void finalize();
    }, maxDurationMs);
  }

  return {
    stream,
    stop: async () => {
      try {
        return await finalize();
      } catch (error) {
        rejectCompleted?.(error);
        throw error;
      }
    },
    completed,
  };
}

export async function startWavCapture(): Promise<ActiveAudioCapture> {
  const capture = await createPcmCapture();
  return {
    stream: capture.stream,
    stop: capture.stop,
  };
}

export async function startSilenceAwareWavCapture(options?: PcmCaptureOptions): Promise<SilenceAwareAudioCapture> {
  const capture = await createPcmCapture({
    silenceDurationMs: options?.silenceDurationMs ?? 1200,
    minSpeechDurationMs: options?.minSpeechDurationMs ?? 600,
    maxDurationMs: options?.maxDurationMs ?? 20000,
    silenceThreshold: options?.silenceThreshold ?? 0.012,
  });
  return capture;
}
