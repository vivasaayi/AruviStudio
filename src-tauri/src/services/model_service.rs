use crate::domain::model::ModelProvider;
use crate::error::AppError;
use crate::providers::gateway::ModelGateway;
use crate::providers::types::{CompletionRequest, CompletionResponse};
use crate::secrets;
use aruvi_model_client::{anthropic::AnthropicClient, openai_compatible::{OpenAiCompatibleClient, CompletionOptions, TemperatureMode, TokenLimitField}};
use aruvi_model_client::{ClientOptions, ModelGateway as SharedModelGateway};
use async_trait::async_trait;
use sqlx::SqlitePool;
use std::{sync::Arc, time::Duration};

pub struct ModelService;

impl ModelService {
    pub fn new(_db: Arc<SqlitePool>) -> Self {
        Self
    }

    /// Create a model gateway for a provider
    pub fn create_gateway(
        &self,
        provider: &ModelProvider,
    ) -> Result<Box<dyn ModelGateway>, AppError> {
        create_gateway(provider)
    }
}

struct SharedGatewayAdapter(Box<dyn SharedModelGateway>);

#[async_trait]
impl ModelGateway for SharedGatewayAdapter {
    async fn health_check(&self) -> Result<bool, AppError> {
        self.0
            .health_check()
            .await
            .map_err(AppError::from)
    }

    async fn list_models_page(&self, cursor: Option<&str>) -> Result<aruvi_model_client::ModelPage, AppError> {
        self.0.list_models_page(cursor).await.map_err(AppError::from)
    }

    async fn run_completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, AppError> {
        validate_text_model_name(&request.model)?;
        self.0
            .run_completion(request)
            .await
            .map_err(AppError::from)
    }

    async fn run_json_completion(&self, request: CompletionRequest) -> Result<CompletionResponse, AppError> {
        validate_text_model_name(&request.model)?;
        self.0.run_json_completion(request).await.map_err(AppError::from)
    }
}

pub(crate) fn validate_text_model_name(name: &str) -> Result<(), AppError> {
    if crate::services::speech_service::is_voice_only_selector(name) {
        return Err(AppError::Validation("Voice-only selectors cannot be used for text completion".into()));
    }
    Ok(())
}

/// Select the transport from the saved provider protocol, resolving credentials
/// through Studio's existing secret store.
pub fn create_gateway(provider: &ModelProvider) -> Result<Box<dyn ModelGateway>, AppError> {
    Ok(Box::new(SharedGatewayAdapter(create_shared_gateway(provider)?)))
}

// These service-wide bounds also apply to desktop, MCP, agents and remote voice.
// Keep parsing separate from process environment access so validation is testable
// without mutating environment variables shared by concurrent tests.
fn model_client_options(connect: Option<&str>, request: Option<&str>) -> Result<ClientOptions, AppError> {
    fn seconds(value: Option<&str>, name: &str, default: Duration) -> Result<Duration, AppError> {
        match value {
            None => Ok(default),
            Some(value) => value.parse::<u64>().ok().filter(|value| *value > 0 && *value <= 86_400)
                .map(Duration::from_secs)
                .ok_or_else(|| AppError::Validation(format!("{name} must be a whole number of seconds between 1 and 86400"))),
        }
    }
    let defaults = ClientOptions::default();
    let options = ClientOptions {
        connect_timeout: seconds(connect, "ARUVI_MODEL_CONNECT_TIMEOUT_SECS", defaults.connect_timeout)?,
        request_timeout: seconds(request, "ARUVI_MODEL_REQUEST_TIMEOUT_SECS", defaults.request_timeout)?,
    };
    if options.connect_timeout > options.request_timeout {
        return Err(AppError::Validation("Model connection timeout must not exceed request timeout".into()));
    }
    Ok(options)
}

fn configured_model_client_options() -> Result<ClientOptions, AppError> {
    fn read(name: &str) -> Result<Option<String>, AppError> {
        match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(AppError::Validation(format!("{name} must be valid text"))),
        }
    }
    let connect = read("ARUVI_MODEL_CONNECT_TIMEOUT_SECS")?;
    let request = read("ARUVI_MODEL_REQUEST_TIMEOUT_SECS")?;
    model_client_options(connect.as_deref(), request.as_deref())
}

pub(crate) fn create_deepseek_voice_client(provider: &ModelProvider) -> Result<aruvi_model_client::deepseek::DeepSeekVoiceClient, AppError> {
    if !provider.enabled { return Err(AppError::Validation("Selected provider is disabled".into())); }
    if provider.provider_type.as_str() != "openai_compatible" || provider.protocol != "openai_compatible" {
        return Err(AppError::Validation("DeepSeek voice requires an OpenAI-compatible provider".into()));
    }
    let key = secrets::resolve_provider_secret(provider)?
        .ok_or_else(|| AppError::Validation("A DeepSeek API key is required".into()))?;
    aruvi_model_client::deepseek::DeepSeekVoiceClient::with_options(provider.base_url.clone(), key, configured_model_client_options()?)
        .map_err(|error| AppError::Validation(error.to_string()))
}

/// Build one shared transport for chat, streaming, discovery and remote voice.
pub fn stream_usage_setting_key(provider_id: &str) -> String {
    format!("model.provider.{provider_id}.stream_usage")
}

pub async fn create_stream_gateway(provider: &ModelProvider, db: &SqlitePool) -> Result<Box<dyn SharedModelGateway>, AppError> {
    let enabled = if provider.protocol == "openai_compatible" && provider.provider_type.as_str() == "openai_compatible" {
        match crate::persistence::settings_repo::get_setting(db, &stream_usage_setting_key(&provider.id)).await?.as_deref() {
            None | Some("false") => false,
            Some("true") => true,
            _ => return Err(AppError::Validation("Invalid provider stream usage setting".into())),
        }
    } else { false };
    create_shared_gateway_inner(provider, enabled)
}

pub fn create_shared_gateway(provider: &ModelProvider) -> Result<Box<dyn SharedModelGateway>, AppError> {
    create_shared_gateway_inner(provider, false)
}

fn create_shared_gateway_inner(provider: &ModelProvider, include_stream_usage: bool) -> Result<Box<dyn SharedModelGateway>, AppError> {
    if !provider.enabled {
        return Err(AppError::Validation("Selected provider is disabled".into()));
    }
    if provider.provider_type.as_str() != "openai_compatible" {
        return Err(AppError::Validation(format!(
            "Unsupported provider type: {}",
            provider.provider_type
        )));
    }
    if provider.protocol == "anthropic"
        && provider
            .auth_secret_ref
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(AppError::Validation(
            "An explicit Anthropic API key is required".into(),
        ));
    }
    let api_key = secrets::resolve_provider_secret(provider)?;
    create_transport_configured(provider, api_key, configured_model_client_options()?, include_stream_usage)
}

#[cfg(test)]
fn create_transport(provider: &ModelProvider, api_key: Option<String>, options: ClientOptions) -> Result<Box<dyn SharedModelGateway>, AppError> {
    create_transport_configured(provider, api_key, options, false)
}

fn create_transport_configured(provider: &ModelProvider, api_key: Option<String>, options: ClientOptions, include_stream_usage: bool) -> Result<Box<dyn SharedModelGateway>, AppError> {
    crate::persistence::model_repo::validate_provider_protocol(provider.provider_type.as_str(), &provider.protocol)?;
    crate::persistence::model_repo::validate_completion_profile(provider.provider_type.as_str(), &provider.protocol, &provider.completion_profile)?;
    match provider.protocol.as_str() {
        "openai_compatible" => {
            let client = OpenAiCompatibleClient::with_options(provider.base_url.clone(), api_key, options)
                .map_err(|error| AppError::Validation(error.to_string()))?;
            let completion_options = match provider.completion_profile.as_str() {
                "standard" => CompletionOptions::default(),
                "openai_reasoning" => CompletionOptions { token_limit_field: TokenLimitField::MaxCompletionTokens, temperature: TemperatureMode::Omit },
                _ => return Err(AppError::Validation("Unsupported provider completion profile".into())),
            };
            Ok(Box::new(client.with_completion_options(completion_options).with_stream_usage(include_stream_usage)))
        },
        "anthropic" => {
            let key = api_key
                .ok_or_else(|| AppError::Validation("Anthropic API key is required".into()))?;
            let client = AnthropicClient::with_options(provider.base_url.clone(), key, options)
                .map_err(|error| AppError::Validation(error.to_string()))?;
            Ok(Box::new(client))
        }
        protocol => Err(AppError::Validation(format!(
            "Unsupported provider protocol: {protocol}"
        ))),
    }
}

#[cfg(test)]
mod timeout_configuration_tests {
    use super::*;

    #[tokio::test]
    async fn stream_gateway_applies_provider_setting() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT DEFAULT CURRENT_TIMESTAMP)").execute(&pool).await.unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut data=Vec::new(); let mut buf=[0;4096];
            let end=loop { let n=socket.read(&mut buf).unwrap(); assert_ne!(n,0); data.extend_from_slice(&buf[..n]); if let Some(i)=data.windows(4).position(|v|v==b"\r\n\r\n"){break i+4;} };
            let headers=String::from_utf8_lossy(&data[..end]).to_lowercase();
            let len:usize=headers.lines().find_map(|line|line.strip_prefix("content-length: ")).unwrap().trim().parse().unwrap();
            while data.len()<end+len { let n=socket.read(&mut buf).unwrap(); assert_ne!(n,0); data.extend_from_slice(&buf[..n]); }
            let body:serde_json::Value=serde_json::from_slice(&data[end..end+len]).unwrap();
            assert_eq!(body["stream_options"]["include_usage"], true);
            let events="data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
            write!(socket,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",events.len(),events).unwrap();
        });
        let provider=ModelProvider { id:"stream-fixture".into(), name:"Fixture".into(), provider_type:crate::domain::model::ProviderType::OpenaiCompatible, protocol:"openai_compatible".into(), completion_profile:"standard".into(), base_url:format!("http://{address}"), auth_secret_ref:None, enabled:true, created_at:"".into(), updated_at:"".into() };
        crate::persistence::settings_repo::set_setting(&pool, &stream_usage_setting_key(&provider.id), "true").await.unwrap();
        let client=create_stream_gateway(&provider,&pool).await.unwrap();
        let response=client.stream_completion(aruvi_model_client::CompletionRequest { model:"fixture".into(), messages:vec![aruvi_model_client::ChatMessage { role:"user".into(), content:"hello".into() }], temperature:None, max_tokens:Some(5) }, &mut |_| {}).await.unwrap();
        assert_eq!(response.content,"ok"); server.join().unwrap();
        crate::persistence::settings_repo::set_setting(&pool, &stream_usage_setting_key(&provider.id), "invalid").await.unwrap();
        assert!(matches!(create_stream_gateway(&provider,&pool).await, Err(AppError::Validation(_))));
        pool.close().await;
    }

    #[test]
    fn disabled_providers_are_rejected_before_credentials_or_gateway_creation() {
        let mut provider = ModelProvider { id: "fixture".into(), name: "fixture".into(),
            provider_type: crate::domain::model::ProviderType::OpenaiCompatible,
            protocol: "anthropic".into(), completion_profile: "standard".into(), base_url: "not a valid URL".into(),
            auth_secret_ref: None, enabled: false, created_at: "".into(), updated_at: "".into() };
        for protocol in ["anthropic", "openai_compatible"] {
            provider.protocol = protocol.into();
            assert!(matches!(create_shared_gateway(&provider), Err(AppError::Validation(message)) if message == "Selected provider is disabled"));
            assert!(matches!(create_gateway(&provider), Err(AppError::Validation(message)) if message == "Selected provider is disabled"));
        }
    }

    #[tokio::test]
    async fn configured_timeout_interrupts_stalled_provider_response() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buffer = [0; 4096];
            let _ = socket.read(&mut buffer);
            let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 50\r\nConnection: close\r\n\r\n");
            std::thread::sleep(Duration::from_millis(1500));
        });
        let provider = ModelProvider { id: "fixture".into(), name: "fixture".into(),
            provider_type: crate::domain::model::ProviderType::OpenaiCompatible,
            protocol: "openai_compatible".into(), completion_profile: "standard".into(), base_url: format!("http://{address}"),
            auth_secret_ref: None, enabled: true, created_at: "".into(), updated_at: "".into() };
        let options = model_client_options(Some("1"), Some("1")).unwrap();
        let client = create_transport(&provider, None, options).unwrap();
        let result = client.run_completion(CompletionRequest { model: "fixture".into(),
            messages: vec![crate::providers::types::ChatMessage { role: "user".into(), content: "hi".into() }],
            temperature: None, max_tokens: Some(4) }).await;
        assert!(result.is_err());
        assert!(!server.is_finished(), "the request must time out before the server closes");
        server.join().unwrap();
    }

    #[test]
    fn accepts_defaults_and_configured_bounds() {
        let defaults = model_client_options(None, None).unwrap();
        assert_eq!((defaults.connect_timeout.as_secs(), defaults.request_timeout.as_secs()), (15, 300));
        let custom = model_client_options(Some("20"), Some("3600")).unwrap();
        assert_eq!((custom.connect_timeout.as_secs(), custom.request_timeout.as_secs()), (20, 3600));
        assert_eq!(model_client_options(Some("1"), Some("86400")).unwrap().request_timeout.as_secs(), 86400);
    }

    #[test]
    fn rejects_invalid_or_inconsistent_bounds() {
        for (connect, request) in [(Some("0"), None), (Some("nan"), None), (Some(" 1"), None),
            (None, Some("86401")), (Some("30"), Some("29")), (None, Some("1"))] {
            assert!(model_client_options(connect, request).is_err(), "{connect:?} {request:?}");
        }
    }
}
