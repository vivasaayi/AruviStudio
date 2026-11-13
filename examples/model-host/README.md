# Local coding-model validation

`qwen2.5-coder-1.5b-q4.json` is a pinned Model Host registration for the
[Qwen2.5-Coder 1.5B Instruct GGUF](https://huggingface.co/Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF).
The model is Apache-2.0 licensed. Its Q4_K_M artifact is approximately 1.12 GB;
the fixture requests a 2 GB host reservation and a 4096-token context. It is a
small validation model, not a claim that the model is suitable for all coding
or production workloads. The model file is downloaded and checksum-verified by
Model Host and is never stored in this repository.

`qwen2.5-coder-7b-q4.json` pins the official
[7B coding model](https://huggingface.co/Qwen/Qwen2.5-Coder-7B-Instruct-GGUF).
Its artifact is approximately 4.68 GB; it requests a 7 GB reservation and an
8192-token context. Ensure the isolated host's configured memory budget and
available system memory accommodate it before loading. It is also an evaluation
fixture until it passes the workflow checks.

Run these commands only against a disposable native Model Host worktree with
available worker capacity. Replace the paths with the actual worktrees:

```sh
cd /path/to/aruvi-model-host-worktree
ARUVI_MODEL_HOST_ENV=native python3 scripts/client.py worktree register /path/to/AruviStudio/examples/model-host/qwen2.5-coder-1.5b-q4.json
ARUVI_MODEL_HOST_ENV=native python3 scripts/client.py worktree install qwen25-coder-15b-q4
# Wait until `models` reports installation: installed.
ARUVI_MODEL_HOST_ENV=native python3 scripts/client.py worktree load qwen25-coder-15b-q4
cd /path/to/AruviStudio
scripts/configure-native-model-host /path/to/aruvi-model-host-worktree
bash scripts/test host-coding
bash scripts/test host-coding-ui
```

The setup helper uses the worktree's private token without printing it, checks
that the model is installed, and creates or refreshes a named Studio provider
and model definition. It does not assign the model to agents. Native worktree
ports are dynamic; rerun the helper after the host restarts. The host's provider
credential has broad host authority and should be scoped more narrowly when the
host supports it.

To evaluate the larger model, register and install the 7B JSON, then configure
Studio with `--model qwen25-coder-7b-q4` and set
`ARUVI_CODING_TEST_MODEL=qwen25-coder-7b-q4` for the test commands. The host
must have sufficient configured reservation capacity; do not unload unrelated
workers to make room.

`scripts/test-host-coding-workflow /path/to/aruvi-model-host-worktree` is a stronger, potentially slower
quality check. It creates a disposable Studio fixture and repository, binds its
agents to this model, runs the approval workflow, and checks generated JavaScript
in a network-isolated container. It does not modify the running Studio database
or the existing staging host. The check may fail for a model that can answer a
short coding prompt but cannot complete the agent workflow; treat that result as
an evaluation finding rather than a transport failure.
