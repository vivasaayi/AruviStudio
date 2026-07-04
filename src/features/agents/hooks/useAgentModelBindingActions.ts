import type { Dispatch, SetStateAction } from "react";

import type { AgentDefinition, ModelDefinition } from "../../../lib/types";

type AgentModelBindingMutation = {
  mutateAsync: (payload: { agentId: string; modelId: string }) => Promise<unknown>;
};

type AgentModelBindingActionsInput = {
  agents: AgentDefinition[];
  modelDefinitions: ModelDefinition[];
  bindAgentModelMutation: AgentModelBindingMutation;
  setAgentFeedback: Dispatch<SetStateAction<string | null>>;
  setAgentError: Dispatch<SetStateAction<string | null>>;
};

export function useAgentModelBindingActions({
  agents,
  modelDefinitions,
  bindAgentModelMutation,
  setAgentFeedback,
  setAgentError,
}: AgentModelBindingActionsInput) {
  const bindAgentsToModel = async (agentsToBind: AgentDefinition[], model: ModelDefinition) => {
    await Promise.all(
      agentsToBind.map((agent) =>
        bindAgentModelMutation.mutateAsync({
          agentId: agent.id,
          modelId: model.id,
        }),
      ),
    );
  };

  const bindCodingAgentsToDeepSeek = async () => {
    setAgentError(null);
    setAgentFeedback(null);
    const deepSeekModel =
      modelDefinitions.find((model) => model.name.toLowerCase().includes("deepseek-coder")) ??
      modelDefinitions.find((model) => model.name.toLowerCase().includes("deepseek"));
    if (!deepSeekModel) {
      setAgentError("No DeepSeek model definition found. Add one in the Models tab first.");
      return;
    }

    const codingAgents = agents.filter((agent) => {
      const role = agent.role.toLowerCase();
      return agent.enabled && (role.includes("coding") || role.includes("developer"));
    });

    if (codingAgents.length === 0) {
      setAgentError("No enabled coding/developer agents found.");
      return;
    }

    try {
      await bindAgentsToModel(codingAgents, deepSeekModel);
      setAgentFeedback(`Bound ${codingAgents.length} coding agents to ${deepSeekModel.name}.`);
    } catch {
      // Mutation handler sets feedback.
    }
  };

  const bindAllAgentsToDeepSeek = async () => {
    setAgentError(null);
    setAgentFeedback(null);
    const deepSeekModel =
      modelDefinitions.find((model) => model.enabled && model.name.toLowerCase().includes("deepseek-coder")) ??
      modelDefinitions.find((model) => model.enabled && model.name.toLowerCase().includes("deepseek"));
    if (!deepSeekModel) {
      setAgentError("No DeepSeek model definition found. Add one in the Models tab first.");
      return;
    }

    const enabledAgents = agents.filter(
      (agent) => agent.enabled && agent.employment_status === "active",
    );

    if (enabledAgents.length === 0) {
      setAgentError("No active enabled agents found.");
      return;
    }

    try {
      await bindAgentsToModel(enabledAgents, deepSeekModel);
      setAgentFeedback(`Bound ${enabledAgents.length} enabled agents to ${deepSeekModel.name}.`);
      setAgentError(null);
    } catch {
      // Mutation handler sets feedback.
    }
  };

  return {
    bindAllAgentsToDeepSeek,
    bindCodingAgentsToDeepSeek,
  };
}
