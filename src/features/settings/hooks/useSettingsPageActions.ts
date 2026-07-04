import type { QueryClient } from "@tanstack/react-query";
import {
  clearDatabasePathOverride,
  routePlannerContact,
  seedExampleProducts,
  sendTwilioWhatsappMessage,
  setDatabasePathOverride,
  setSetting,
  startTwilioVoiceCall,
} from "../../../lib/tauri";
import { HIDE_EXAMPLE_PRODUCTS_KEY } from "../lib/settingsKeys";
import type { useSettingsPageState } from "./useSettingsPageState";

type SettingsPageState = ReturnType<typeof useSettingsPageState>;

export function useSettingsPageActions({
  queryClient,
  settings,
}: {
  queryClient: QueryClient;
  settings: SettingsPageState;
}) {
  const {
    dbPathOverrideInput,
    plannerContactTarget,
    plannerContactOpeningMessage,
    setCatalogActionError,
    setCatalogActionMsg,
    setDbPathOverrideError,
    setDbPathOverrideInput,
    setDbPathOverrideSaved,
    setPlannerContactError,
    setPlannerContactMsg,
    setSavedMsg,
  } = settings;

  const invalidateProductCatalogQueries = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["products"] }),
      queryClient.invalidateQueries({ queryKey: ["productTree"] }),
      queryClient.invalidateQueries({ queryKey: ["sidebarProductTree"] }),
      queryClient.invalidateQueries({ queryKey: ["inspectorProductTree"] }),
    ]);
  };

  const saveSetting = async (key: string, value: string) => {
    await setSetting(key, value);
    await queryClient.invalidateQueries({ queryKey: ["setting"] });
    await queryClient.invalidateQueries({ queryKey: ["mcpBridgeStatus"] });
    await queryClient.invalidateQueries({ queryKey: ["mobileBridgeStatus"] });
    if (key === HIDE_EXAMPLE_PRODUCTS_KEY) {
      await invalidateProductCatalogQueries();
    }
    setSavedMsg(key);
    setTimeout(() => setSavedMsg(null), 2000);
  };

  const seedExampleCatalog = async () => {
    try {
      setCatalogActionError(null);
      await seedExampleProducts();
      await Promise.all([
        invalidateProductCatalogQueries(),
        queryClient.invalidateQueries({ queryKey: ["workItems"] }),
      ]);
      setCatalogActionMsg("Example catalog is present and up to date.");
      setTimeout(() => setCatalogActionMsg(null), 2500);
    } catch (error) {
      setCatalogActionError(String(error));
    }
  };

  const saveDbOverride = async () => {
    try {
      setDbPathOverrideError(null);
      await setDatabasePathOverride(dbPathOverrideInput);
      setDbPathOverrideSaved("saved");
      setTimeout(() => setDbPathOverrideSaved(null), 2500);
    } catch (error) {
      setDbPathOverrideError(String(error));
    }
  };

  const clearDbOverride = async () => {
    try {
      setDbPathOverrideError(null);
      await clearDatabasePathOverride();
      setDbPathOverrideInput("");
      setDbPathOverrideSaved("cleared");
      setTimeout(() => setDbPathOverrideSaved(null), 2500);
    } catch (error) {
      setDbPathOverrideError(String(error));
    }
  };

  const copyText = async (value: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setSavedMsg(`copied:${value}`);
      setTimeout(() => setSavedMsg(null), 2000);
    } catch {
      setSavedMsg(null);
    }
  };

  const autoRoutePlannerContact = async () => {
    try {
      setPlannerContactError(null);
      setPlannerContactMsg(null);
      const result = await routePlannerContact({
        to: plannerContactTarget.trim(),
        content: plannerContactOpeningMessage.trim(),
      });
      const channelLabel = result.channel === "voice" ? "voice call" : "WhatsApp";
      if (result.status === "blocked") {
        setPlannerContactError(`Auto-routing blocked: ${result.reason}`);
        return;
      }
      setPlannerContactMsg(`Auto-routed to ${channelLabel}. ${result.reason}`);
    } catch (error) {
      setPlannerContactError(String(error));
    }
  };

  const sendPlannerWhatsapp = async () => {
    try {
      setPlannerContactError(null);
      setPlannerContactMsg(null);
      await sendTwilioWhatsappMessage({
        to: plannerContactTarget.trim(),
        content: plannerContactOpeningMessage.trim(),
      });
      setPlannerContactMsg("WhatsApp message queued through Twilio.");
    } catch (error) {
      setPlannerContactError(String(error));
    }
  };

  const startPlannerVoiceCall = async () => {
    try {
      setPlannerContactError(null);
      setPlannerContactMsg(null);
      await startTwilioVoiceCall({
        to: plannerContactTarget.trim(),
        initialPrompt: plannerContactOpeningMessage.trim() || undefined,
      });
      setPlannerContactMsg("Voice call requested through Twilio.");
    } catch (error) {
      setPlannerContactError(String(error));
    }
  };

  return {
    autoRoutePlannerContact,
    clearDbOverride,
    copyText,
    saveDbOverride,
    saveSetting,
    seedExampleCatalog,
    sendPlannerWhatsapp,
    startPlannerVoiceCall,
  };
}
