import type { QueryClient } from "@tanstack/react-query";

const agentRegistryQueryKeys = [
  ["agents"],
  ["agent-teams"],
  ["agent-team-memberships"],
  ["agent-team-assignments"],
  ["agent-model-bindings"],
  ["skills"],
  ["agent-skill-links"],
  ["team-skill-links"],
  ["workflow-stage-policies"],
  ["model-definitions"],
] as const;

const agentRegistryActiveQueryKeys = [
  ["agents"],
  ["agent-teams"],
  ["agent-team-memberships"],
  ["agent-team-assignments"],
] as const;

export async function invalidateAgentRegistryData(queryClient: QueryClient) {
  await Promise.all([
    ...agentRegistryQueryKeys.map((queryKey) => queryClient.invalidateQueries({ queryKey })),
    ...agentRegistryActiveQueryKeys.map((queryKey) =>
      queryClient.refetchQueries({ queryKey, type: "active" }),
    ),
  ]);
}
