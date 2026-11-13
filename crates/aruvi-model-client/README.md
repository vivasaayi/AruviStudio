# Aruvi Model Client

This crate is the initial extraction of Studio's OpenAI-compatible HTTP client.
It owns model request and response types and the transport, while the application
continues to own credentials, logs, telemetry, model configuration and workflows.
The crate has no dependency on Studio or Tauri.

The current implementation covers non-streaming chat completions, a buffered
SSE fallback, remote multipart audio transcription and bounded WAV speech generation.
Native Anthropic, incremental streaming, general response limits, configuration
policy and standalone distribution are not yet implemented.
This repository location is provisional until the shared versioning and publication
strategy is validated. The repository's PolyForm Noncommercial license applies.
