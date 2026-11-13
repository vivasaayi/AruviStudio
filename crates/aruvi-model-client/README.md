# Aruvi Model Client

This crate is the initial extraction of Studio's OpenAI-compatible HTTP client.
It owns model request and response types and the transport, while the application
continues to own credentials, logs, telemetry, model configuration and workflows.
The crate has no dependency on Studio or Tauri.

The current implementation covers non-streaming OpenAI-compatible completions,
a buffered SSE fallback, remote multipart audio transcription, bounded WAV speech
generation, and a native Anthropic Messages text client. Studio does not yet route
providers to the Anthropic client. Incremental streaming, general response limits,
configuration policy and standalone distribution are not yet implemented.
This repository location is provisional until the shared versioning and publication
strategy is validated. The repository's PolyForm Noncommercial license applies.

The native Anthropic adapter follows the [Messages API](https://platform.claude.com/docs/en/api/messages/create)
and [authentication](https://platform.claude.com/docs/en/manage-claude/authentication)
contracts. It handles text only and rejects tool-use responses; it does not yet
support incremental streaming or thinking/tool blocks.
