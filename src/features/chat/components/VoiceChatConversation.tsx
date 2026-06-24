import { styles } from "../lib/voiceChatPageStyles";
import type { LocalChatMessage } from "../lib/voiceChatTypes";

interface VoiceChatConversationProps {
  messages: LocalChatMessage[];
}

export function VoiceChatConversation({ messages }: VoiceChatConversationProps) {
  return (
    <div style={styles.conversation} data-testid="voice-chat-conversation">
      {messages.length === 0 ? (
        <div style={styles.helper}>Start a voice session and say something. The conversation transcript will accumulate here.</div>
      ) : (
        messages.map((message) => (
          <div key={message.id} style={message.role === "user" ? styles.bubbleUser : styles.bubbleAssistant}>
            <div style={styles.bubbleMeta}>{message.role === "user" ? "You said" : "Assistant replied"}</div>
            {message.content}
          </div>
        ))
      )}
    </div>
  );
}
