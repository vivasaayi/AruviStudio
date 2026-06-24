export const AUTO_START_AFTER_APPROVAL_KEY = "workflow.auto_start_after_work_item_approval";
export const AUTO_APPROVE_PLAN_KEY = "workflow.auto_approve_plan";
export const AUTO_APPROVE_TEST_REVIEW_KEY = "workflow.auto_approve_test_review";
export const HIDE_EXAMPLE_PRODUCTS_KEY = "catalog.hide_example_products";
export const PLANNER_DEFAULT_PROVIDER_KEY = "planner.default_provider_id";
export const PLANNER_DEFAULT_MODEL_KEY = "planner.default_model_name";
export const PLANNER_CHANNEL_PREFERENCE_KEY = "planner.channel_preference";
export const PLANNER_ESCALATE_TO_CALL_KEY = "planner.escalate_to_call_on_ambiguity";
export const PLANNER_CALL_QUIET_HOURS_START_KEY = "planner.call_quiet_hours_start";
export const PLANNER_CALL_QUIET_HOURS_END_KEY = "planner.call_quiet_hours_end";
export const SPEECH_PROVIDER_KEY = "speech.transcription_provider_id";
export const SPEECH_MODEL_KEY = "speech.transcription_model_name";
export const SPEECH_LOCALE_KEY = "speech.locale";
export const SPEECH_NATIVE_VOICE_KEY = "speech.native_voice";
export const SPEECH_ENABLE_MIC_KEY = "speech.enable_mic";
export const SPEECH_AUTO_SPEAK_REPLIES_KEY = "speech.auto_speak_replies";
export const SPEECH_REVIEW_BEFORE_SEND_KEY = "speech.review_before_send";
export const MCP_API_TOKEN_KEY = "mcp.api_token";
export const MOBILE_API_TOKEN_KEY = "mobile.api_token";
export const MOBILE_BIND_HOST_KEY = "mobile.bind_host";
export const MOBILE_BIND_PORT_KEY = "mobile.bind_port";
export const TWILIO_ACCOUNT_SID_KEY = "twilio.account_sid";
export const TWILIO_AUTH_TOKEN_KEY = "twilio.auth_token";
export const TWILIO_WHATSAPP_FROM_KEY = "twilio.whatsapp_from";
export const TWILIO_VOICE_FROM_KEY = "twilio.voice_from";
export const TWILIO_WEBHOOK_BASE_URL_KEY = "twilio.webhook_base_url";
export const PLANNER_CONTACT_TARGET_KEY = "planner.contact_target";
export const PLANNER_CONTACT_OPENING_MESSAGE_KEY = "planner.contact_opening_message";

export function parseBooleanSetting(value: string | null | undefined, fallback: boolean) {
  if (value == null) {
    return fallback;
  }
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
