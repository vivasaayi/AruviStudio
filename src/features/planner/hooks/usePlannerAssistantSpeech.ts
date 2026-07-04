import { speakTextNatively } from "../../../lib/tauri";
import { speakInBrowser } from "../../shared/voice";

type UsePlannerAssistantSpeechOptions = {
  speechLocaleSetting: string;
  speechNativeVoiceSetting: string;
};

export function usePlannerAssistantSpeech({
  speechLocaleSetting,
  speechNativeVoiceSetting,
}: UsePlannerAssistantSpeechOptions) {
  const speakAssistantReply = async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) {
      return;
    }
    try {
      await speakTextNatively({
        text: trimmed,
        voice: speechNativeVoiceSetting || undefined,
        locale: speechLocaleSetting || "en-US",
      });
    } catch {
      speakInBrowser(trimmed);
    }
  };

  return { speakAssistantReply };
}
