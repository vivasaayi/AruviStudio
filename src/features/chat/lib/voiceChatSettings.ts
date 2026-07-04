export const SPEECH_PROVIDER_KEY = "speech.transcription_provider_id";
export const SPEECH_MODEL_KEY = "speech.transcription_model_name";
export const SPEECH_LOCALE_KEY = "speech.locale";
export const SPEECH_NATIVE_VOICE_KEY = "speech.native_voice";
export const SPEECH_ENABLE_MIC_KEY = "speech.enable_mic";

export function parseBooleanSetting(value: string | null | undefined, fallback: boolean) {
  if (value == null) return fallback;
  switch (value.trim().toLowerCase()) {
    case "1":
    case "true":
    case "yes":
    case "on":
      return true;
    case "0":
    case "false":
    case "no":
    case "off":
      return false;
    default:
      return fallback;
  }
}

export function stopBrowserSpeech() {
  if (typeof window !== "undefined" && "speechSynthesis" in window) {
    window.speechSynthesis.cancel();
  }
}
