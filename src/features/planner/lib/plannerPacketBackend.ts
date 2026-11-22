import { buildDesignReviewPacketHtml, slugifyPacketName, type DesignReviewPacketInput } from "./plannerReviewPacket";

// Bundled into the backend renderer; the request is an explicit client snapshot.
export function renderJson(requestJson: string): string {
  const input = JSON.parse(requestJson) as DesignReviewPacketInput;
  return JSON.stringify({
    filename: `${slugifyPacketName(input.title)}-design-review-packet.html`,
    html: buildDesignReviewPacketHtml(input).replace(
      '<main class="shell">',
      '<main class="shell"><p role="note">This packet uses a caller-supplied snapshot, which may be stale and has not been checked against the current catalog.</p>',
    ),
  });
}
