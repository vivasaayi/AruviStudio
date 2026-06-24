import type { ChatMessagePayload } from "../../../lib/types";

export type LocalChatMessage = ChatMessagePayload & { id: string };
