import { describeError } from "./mobileFormatters";

export type ConnectionValues = {
  baseUrl?: string;
  token?: string;
  providerId?: string;
  modelName?: string;
  locale?: string;
};

export function parseConnectionUrl(url: string): ConnectionValues | null {
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== "aruvi-planner-mobile:") {
      return null;
    }
    const params = parsed.searchParams;
    const values: ConnectionValues = {
      baseUrl: params.get("baseUrl") || params.get("base_url") || undefined,
      token: params.get("token") || undefined,
      providerId: params.get("providerId") || params.get("provider_id") || undefined,
      modelName: params.get("modelName") || params.get("model_name") || undefined,
      locale: params.get("locale") || undefined,
    };
    return Object.values(values).some(Boolean) ? values : null;
  } catch {
    return null;
  }
}

export function buildRemoteScript(payload: {
  token: string;
  provider: string;
  model: string;
  locale: string;
  activeTab: string;
}) {
  return `
    (function () {
      try {
        var config = ${JSON.stringify(payload)};
        if (config.token) window.localStorage.setItem("aruvi.remote.token", config.token);
        else window.localStorage.removeItem("aruvi.remote.token");
        if (config.provider) window.localStorage.setItem("aruvi.remote.provider", config.provider);
        else window.localStorage.removeItem("aruvi.remote.provider");
        if (config.model) window.localStorage.setItem("aruvi.remote.model", config.model);
        else window.localStorage.removeItem("aruvi.remote.model");
        if (config.locale) window.localStorage.setItem("aruvi.remote.locale", config.locale);
        else window.localStorage.removeItem("aruvi.remote.locale");
        window.localStorage.setItem("aruvi.remote.active_tab", config.activeTab);

        var styleId = "aruvi-native-shell-style";
        var style = document.getElementById(styleId);
        if (!style) {
          style = document.createElement("style");
          style.id = styleId;
          style.textContent = [
            ".topbar,.tabbar{display:none!important}",
            ".shell{min-height:100vh!important;display:block!important}",
            ".main{padding:10px 10px 14px 10px!important}",
            ".tab-panel.active{display:block!important}",
            "body{background:#101214!important}"
          ].join("");
          document.head.appendChild(style);
        }

        var activate = function () {
          var button = document.querySelector('.tab-button[data-tab="' + config.activeTab + '"]');
          if (button) button.click();
        };
        activate();
        window.setTimeout(activate, 150);
        window.setTimeout(activate, 500);
      } catch (error) {}
    })();
    true;
  `;
}

export function buildRemoteVoiceSubmitScript(transcript: string) {
  return `
    (function () {
      try {
        var transcript = ${JSON.stringify(transcript)};
        var voiceTab = document.querySelector('.tab-button[data-tab="voice"]');
        if (voiceTab) voiceTab.click();
        var composer = document.getElementById("voiceComposer");
        if (composer) {
          composer.value = transcript;
          composer.dispatchEvent(new Event("input", { bubbles: true }));
        }
        var sendButton = document.getElementById("voiceSendButton");
        if (sendButton) sendButton.click();
      } catch (error) {}
    })();
    true;
  `;
}

export function normalizeBaseUrlForDisplay(value: string) {
  return value.trim().replace(/\/+$/, "");
}

function isLoopbackHost(hostname: string) {
  return ["localhost", "127.0.0.1", "::1"].includes(hostname.toLowerCase());
}

export function getLoopbackFallbackBaseUrl(value: string) {
  try {
    const parsed = new URL(normalizeBaseUrlForDisplay(value));
    if (isLoopbackHost(parsed.hostname)) return null;
    parsed.hostname = "127.0.0.1";
    return normalizeBaseUrlForDisplay(parsed.toString());
  } catch {
    return null;
  }
}

export function isNetworkRequestFailure(error: unknown) {
  return describeError(error).toLowerCase().includes("network request failed");
}
