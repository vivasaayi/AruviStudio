# Aruvi Model Client

This provisional Rust crate contains reusable model transports without depending on
Studio or Tauri. Applications own provider settings, credentials, telemetry and
workflow policy. The repository's PolyForm Noncommercial license applies.

It supports paged model discovery, OpenAI-compatible text completions and incremental streams, remote
multipart transcription and bounded WAV speech generation. It also implements
Anthropic Messages text completions and streams. Studio selects these protocols
for its chat and agent flows. Streaming bounds the total event data, reports
provider errors and incomplete streams, and cancels the HTTP request when its
future is dropped. Text clients explicitly reject tool-use responses; full tools,
thinking, structured output and cross-provider capability negotiation remain open.
For OpenAI-compatible text responses, a supplied finish reason indicating token
truncation, filtering, tool use or an unknown outcome is returned as an error.
Missing or null finish reasons are accepted for compatible providers, and cannot
prove response completeness. A streaming callback may have received partial text
before the terminal error.
For Anthropic, client tool calls, server-side tool calls and tool-result blocks
are rejected even when the provider reports a completed turn. A streaming
callback may already have received partial text before that error.
Mixed Anthropic content rejects invalid or unsupported blocks instead of returning
partial text as a complete response. Documented `thinking` and `redacted_thinking`
blocks are omitted from text-only responses; tool-use turns remain unsupported.
OpenAI-compatible multipart messages likewise reject unsupported or malformed parts,
and multipart streaming deltas are rejected instead of silently dropped.
See the [Anthropic Messages reference](https://platform.claude.com/docs/en/api/messages/create)
for provider content-block types.
For Anthropic, a supplied stop reason indicating token truncation, a paused turn,
a refusal, tool use, or an unknown outcome is returned as an error instead of a
completed text response. Some compatible proxies omit stop reasons; their absence
is accepted for compatibility, so completeness cannot be inferred from that field.
OpenAI-compatible ordinary, JSON-object and streaming completions validate a
nonempty model and message list, positive explicit token limit, and finite
temperature before making a request. Provider-specific temperature ranges remain
subject to the endpoint.
OpenAI-compatible transcription requires a nonempty model name and audio payload.
The adapter does not impose a provider-wide upload size or format policy; check
the selected provider's limits. OpenAI has deprecated `whisper-1` and recommends
`gpt-transcribe` for new file-transcription integrations; this does not imply that
other OpenAI-compatible providers support that model. See the [OpenAI file
transcription guide](https://developers.openai.com/api/docs/guides/speech-to-text).
OpenAI-compatible completion responses are capped at 8 MiB, transcription and
discovery responses at 2 MiB, and WAV speech at 16 MiB. Anthropic completion and
stream responses are capped at 2 MiB. These limits are fixed for now. Provider
error bodies are not included in errors; the HTTP status remains available in
the message. OpenAI-compatible and Anthropic requests reject redirects and
base URLs containing credentials, queries, or fragments.

Speech responses must have a structurally complete RIFF/WAVE container with a
format chunk and nonempty audio data; common unknown-length streaming headers
are accepted. This checks the container, not whether the codec can be decoded or
whether the audio is intelligible.

Use `Box<dyn ModelGateway>` when a caller needs to switch providers without
matching on concrete client types. Its methods cover text completion, opt-in
JSON-object completion, text streaming, model discovery, remote transcription
and WAV speech. `run_json_completion` sends OpenAI-compatible JSON-object mode
and rejects output that is not a JSON object; it does not validate a schema.
Prompt the model to produce JSON, and enable this only when the selected remote
model supports the parameter. `supports_json_object_completion()` reports transport
implementation support without a network request; the endpoint or selected model
can still reject it. Anthropic returns `UnsupportedOperation` rather
than silently changing formats. Ordinary completions do not send the option. A transport
can return `UnsupportedOperation` for methods it does not implement; for example,
Anthropic's text transport does not implement voice. `transport_capabilities()`
describes the client adapter, not the remote service or model: an
OpenAI-compatible endpoint can still reject a voice request. Credentials,
model selection and telemetry remain the application's responsibility.

By default, connections time out after 15 seconds and the complete request,
including streaming, after five minutes. Other applications can set both with
`ClientOptions` and `OpenAiCompatibleClient::with_options` or
`AnthropicClient::with_options`. Both must be positive, the connection timeout
cannot exceed the total timeout, and the total is capped at 24 hours. A streamed
request can yield partial chunks before failing or timing out; consumers must
check its final result before treating those chunks as a completed response.
Successful completions and streams include optional `CompletionResponse.metadata`:
provider response ID, model, native finish/stop reason and the provider usage
object, when supplied. Stream usage objects are merged across events. No metadata
is returned on an error, and availability depends on the provider. Adding this
field changes Rust struct literals: update callers to set `metadata` or use
`..Default::default()`. JSON without metadata remains compatible.

Call `ModelGateway::list_models_page(None)` to discover a provider's available
models and pass `next_cursor` back to fetch subsequent pages. OpenAI-compatible
discovery returns one page; Anthropic discovery exposes its pagination cursor.
The `metadata` field preserves provider-supplied fields, including Model Host
task labels. Those fields do not imply that the shared client supports every
advertised capability. Studio exposes the same read-only operation as
`aruvi_models` action `discover_provider_models` and in its model setup forms;
discovery never registers a model automatically.

The separate Git-pinned consumer can also be exercised against an already-running
native Model Host from the Studio checkout:

```sh
bash scripts/test host-shared-client /path/to/aruvi-model-host-worktree --model YOUR_INSTALLED_CODING_MODEL_ID
```

The probe builds the pinned dependency with vendored dependencies and no build
network, then runs the binary in a read-only, non-root container. It reads the
host token from its private environment file and passes it only on standard
input. It validates discovery, ordinary and streamed text, and a token-limit
error. Add `--json-mode` to also test JSON-object completion when the chosen
model supports it. The native host token grants whatever authority the host assigned it;
this check does not modify host configuration, register models or prove model
quality. Use `--environment staging` only when that deployment has the selected
model installed and resources available.

`examples/stream_probe.rs` is a minimal OpenAI-compatible stream smoke test. Set
`ARUVI_PROBE_BASE_URL` and `ARUVI_PROBE_MODEL`, then provide the API key on
standard input (an empty input selects no authentication). It prints only chunk
and character counts. Avoid putting keys in command arguments, logs, or Git.

## Consuming from another project

Until a separate repository and release process are established, use the public
Studio Git repository as the shared source and pin an immutable, tested commit:

```toml
[dependencies]
aruvi-model-client = { git = "https://github.com/vivasaayi/AruviStudio.git", rev = "bb31ba263e86ab635397f57b5c18d1f06f5975e9" }
```

This revision is an integration checkpoint, not a release or a promise of API
stability. Commit the consuming application's `Cargo.lock`. For an offline or
network-isolated Docker build, prepare dependencies from that lockfile and include
the resulting `vendor` directory and Cargo configuration in the build context:

```sh
mkdir -p .cargo
cargo vendor --locked vendor > .cargo/config.toml
cargo test --locked --offline
```

The `validation/model-client-consumer` fixture pins this revision and its own
lockfile; `bash scripts/test shared-client-consumer` builds it in Docker, vendors
dependencies and runs its tests with networking disabled. When promoting another
client revision, first push it, update the fixture's Git revision and lockfile,
then rerun the check before updating consumers. Do not use a mutable branch as a
substitute for a commit pin. Vendored sources include their applicable licenses.

This approach still couples client changes to Studio's Git history. A separately
maintained repository, semver policy, release automation, and package publication
remain future work; nothing has been published to a package registry. Cargo's
package listing includes the repository's license file in the package, but the
crate has not been published or independently released.
The Anthropic adapter follows the [Messages API](https://platform.claude.com/docs/en/api/messages/create)
and [streaming](https://platform.claude.com/docs/en/build-with-claude/streaming) contracts.
