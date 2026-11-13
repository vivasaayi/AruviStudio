# Aruvi Model Client

This provisional Rust crate contains reusable model transports without depending on
Studio or Tauri. Applications own provider settings, credentials, telemetry and
workflow policy. The repository's PolyForm Noncommercial license applies.

It supports OpenAI-compatible text completions and incremental streams, remote
multipart transcription and bounded WAV speech generation. It also implements
Anthropic Messages text completions and streams. Studio selects these protocols
for its chat and agent flows. Streaming bounds the total event data, reports
provider errors and incomplete streams, and cancels the HTTP request when its
future is dropped. Text clients explicitly reject tool-use responses; full tools,
thinking, structured output and cross-provider capability negotiation remain open.
The library does not currently apply consistent bounds to every non-streaming
response.

`examples/stream_probe.rs` is a minimal OpenAI-compatible stream smoke test. Set
`ARUVI_PROBE_BASE_URL` and `ARUVI_PROBE_MODEL`, then provide the API key on
standard input (an empty input selects no authentication). It prints only chunk
and character counts. Avoid putting keys in command arguments, logs, or Git.

This crate remains inside the Studio repository and its release/distribution
strategy is not yet settled. Its current license-file reference and dependency
lock must be addressed before treating it as an independently published package.
The Anthropic adapter follows the [Messages API](https://platform.claude.com/docs/en/api/messages/create)
and [streaming](https://platform.claude.com/docs/en/build-with-claude/streaming) contracts.
