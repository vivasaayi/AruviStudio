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
    downloadUrl: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin?download=true",
    sizeLabel: "75 MiB",
    notes: "Fastest local transcription option for lightweight desktop voice testing.",
  },
  {
    id: "whisper-base-en",
    displayName: "Whisper Base English",
    providerName: "Whisper.cpp Base.en (Local)",
    modelName: "whisper-base.en",
    fileName: "ggml-base.en.bin",
    downloadUrl: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin?download=true",
    sizeLabel: "142 MiB",
    notes: "Best default local speech-to-text model for desktop voice planning and chat.",
  },
  {
    id: "whisper-small-en",
    displayName: "Whisper Small English",
    providerName: "Whisper.cpp Small.en (Local)",
    modelName: "whisper-small.en",
    fileName: "ggml-small.en.bin",
    downloadUrl: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin?download=true",
    sizeLabel: "466 MiB",
    notes: "Higher accuracy local transcription model when you can trade more disk and latency.",
  },
] as const;
