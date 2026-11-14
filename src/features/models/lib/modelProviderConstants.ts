export const SPEECH_PROVIDER_KEY = "speech.transcription_provider_id";
export const SPEECH_MODEL_KEY = "speech.transcription_model_name";
export const LEGACY_WHISPER_PLACEHOLDER_PATH = "/absolute/path/to/ggml-base.en.bin";

export const MANAGED_LOCAL_MODELS = [
  {
    id: "whisper-tiny-en",
    displayName: "Whisper Tiny English",
    providerName: "Whisper.cpp Tiny.en (Local)",
    modelName: "whisper-tiny.en",
    fileName: "ggml-tiny.en.bin",
    downloadUrl: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-tiny.en.bin?download=true",
    expectedSha256: "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f",
    sizeLabel: "75 MiB",
    notes: "Fastest local transcription option for lightweight desktop voice testing.",
  },
  {
    id: "whisper-base-en",
    displayName: "Whisper Base English",
    providerName: "Whisper.cpp Base.en (Local)",
    modelName: "whisper-base.en",
    fileName: "ggml-base.en.bin",
    downloadUrl: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-base.en.bin?download=true",
    expectedSha256: "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002",
    sizeLabel: "142 MiB",
    notes: "Best default local speech-to-text model for desktop voice planning and chat.",
  },
  {
    id: "whisper-small-en",
    displayName: "Whisper Small English",
    providerName: "Whisper.cpp Small.en (Local)",
    modelName: "whisper-small.en",
    fileName: "ggml-small.en.bin",
    downloadUrl: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.en.bin?download=true",
    expectedSha256: "c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d",
    sizeLabel: "466 MiB",
    notes: "Higher accuracy local transcription model when you can trade more disk and latency.",
  },
] as const;
