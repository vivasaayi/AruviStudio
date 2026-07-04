import { Platform, TurboModuleRegistry } from "react-native";
import {
  AudioQuality,
  IOSOutputFormat,
  RecordingPresets,
  type RecordingOptions,
} from "expo-audio";
import * as FileSystem from "expo-file-system";

export type WhisperModelOption = {
  id: string;
  label: string;
  fileName: string;
  sizeLabel: string;
  description: string;
  url: string;
};

export type InstalledWhisperModel = {
  id: string;
  uri: string;
  fileName: string;
  installedAt: string;
  sizeBytes?: number;
};

export const WHISPER_MODELS: WhisperModelOption[] = [
  {
    id: "tiny-en-q5_1",
    label: "Whisper tiny.en Q5",
    fileName: "ggml-tiny.en-q5_1.bin",
    sizeLabel: "31 MB",
    description: "Fastest install and best first test for phone voice chat.",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en-q5_1.bin",
  },
  {
    id: "tiny-en",
    label: "Whisper tiny.en",
    fileName: "ggml-tiny.en.bin",
    sizeLabel: "75 MB",
    description: "Small English model with better quality than the quantized tiny file.",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
  },
  {
    id: "base-en-q5_1",
    label: "Whisper base.en Q5",
    fileName: "ggml-base.en-q5_1.bin",
    sizeLabel: "57 MB",
    description: "Better accuracy while staying reasonable for mobile storage.",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en-q5_1.bin",
  },
];

export const VOICE_RECORDING_OPTIONS: RecordingOptions = Platform.OS === "ios"
  ? {
      extension: ".wav",
      sampleRate: 16000,
      numberOfChannels: 1,
      bitRate: 256000,
      android: RecordingPresets.HIGH_QUALITY.android,
      ios: {
        extension: ".wav",
        outputFormat: IOSOutputFormat.LINEARPCM,
        audioQuality: AudioQuality.MAX,
        sampleRate: 16000,
        linearPCMBitDepth: 16,
        linearPCMIsBigEndian: false,
        linearPCMIsFloat: false,
      },
      web: {
        mimeType: "audio/wav",
      },
    }
  : RecordingPresets.HIGH_QUALITY;

export function parseInstalledWhisperModels(raw: string | null): Record<string, InstalledWhisperModel> {
  if (!raw) return {};
  try {
    const parsed = JSON.parse(raw) as Record<string, InstalledWhisperModel>;
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

export function normalizeWhisperLanguage(locale: string) {
  const normalized = locale.trim().toLowerCase();
  if (!normalized) return "auto";
  return normalized.split(/[-_]/)[0] || "auto";
}

export function whisperModelDirectory() {
  if (!FileSystem.documentDirectory) {
    throw new Error("App document storage is unavailable on this device.");
  }
  return `${FileSystem.documentDirectory}models/whisper/`;
}

export function assertWhisperNativeModuleAvailable() {
  if (!TurboModuleRegistry.get("RNWhisper")) {
    throw new Error(
      "On-device Whisper is not available in this app build. Rebuild and reinstall the Expo dev app after installing whisper.rn.",
    );
  }
}
