import { buildDesignReviewPacketHtml, slugifyPacketName, type DesignReviewPacketInput } from "./plannerReviewPacket";

// Bundled into the backend renderer; the request is an explicit client snapshot.
export function renderJson(requestJson: string): string {
  const input = JSON.parse(requestJson) as DesignReviewPacketInput;
  return JSON.stringify({
    filename: `${slugifyPacketName(input.title)}-design-review-packet.html`,
    html: buildDesignReviewPacketHtml(input).replace(
      '<main class="shell">',
      input.snapshotKind === "proposal"
        ? '<main class="shell"><p role="note">This packet uses a caller-supplied proposal snapshot. The proposal and any recorded draft are from the selected message; catalog context was gathered at export time and may have changed.</p>'
        : '<main class="shell"><p role="note">This packet uses a caller-supplied snapshot, which may be stale and has not been checked against the current catalog.</p>',
    ),
  });
}
