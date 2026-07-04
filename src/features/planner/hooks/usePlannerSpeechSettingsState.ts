import { useEffect, useState } from "react";

import { loadPlannerSpeechSettings } from "../lib/plannerSpeechSettings";

export function usePlannerSpeechSettingsState() {
  const [voiceEnabled, setVoiceEnabled] = useState(true);
  const [autoSpeak, setAutoSpeak] = useState(false);
  const [speechProviderSetting, setSpeechProviderSetting] = useState("");
  const [speechModelSetting, setSpeechModelSetting] = useState("");
  const [speechLocaleSetting, setSpeechLocaleSetting] = useState("en-US");
  const [speechNativeVoiceSetting, setSpeechNativeVoiceSetting] = useState("");
  const [reviewVoiceBeforeSend, setReviewVoiceBeforeSend] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void loadPlannerSpeechSettings().then((settings) => {
      if (cancelled) {
        return;
      }
      if (settings.provider) {
        setSpeechProviderSetting(settings.provider);
      }
      if (settings.model) {
        setSpeechModelSetting(settings.model);
      }
      if (settings.locale) {
        setSpeechLocaleSetting(settings.locale);
      }
      if (settings.nativeVoice) {
        setSpeechNativeVoiceSetting(settings.nativeVoice);
      }
      if (typeof settings.micEnabled === "boolean") {
        setVoiceEnabled(settings.micEnabled);
      }
      if (typeof settings.autoSpeak === "boolean") {
        setAutoSpeak(settings.autoSpeak);
      }
      if (typeof settings.reviewBeforeSend === "boolean") {
        setReviewVoiceBeforeSend(settings.reviewBeforeSend);
      }
    }).catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  return {
    voiceEnabled,
    autoSpeak,
    speechProviderSetting,
    speechModelSetting,
    speechLocaleSetting,
    speechNativeVoiceSetting,
    reviewVoiceBeforeSend,
  };
}
