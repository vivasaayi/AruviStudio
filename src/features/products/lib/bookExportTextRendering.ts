export function renderRichTextHtml(text: string) {
  const normalized = text.replace(/\r\n/g, "\n").trim();
  if (!normalized) {
    return "";
  }

  const lines = normalized.split("\n");
  const blocks: string[] = [];
  let index = 0;

  while (index < lines.length) {
    const line = lines[index];
    const trimmed = line.trim();

    if (!trimmed) {
      index += 1;
      continue;
    }

    if (trimmed.startsWith("```")) {
      const language = trimmed.slice(3).trim();
      index += 1;
      const codeLines: string[] = [];
      while (index < lines.length && !lines[index].trim().startsWith("```")) {
        codeLines.push(lines[index]);
        index += 1;
      }
      if (index < lines.length) {
        index += 1;
      }
      blocks.push(`
        <figure class="book-code">
          <pre><code>${escapeHtml(codeLines.join("\n"))}</code></pre>
          ${language ? `<figcaption>${escapeHtml(language)} snippet</figcaption>` : ""}
        </figure>
      `);
      continue;
    }

    if (isFigureLine(trimmed)) {
      blocks.push(renderFigureBlock(trimmed));
      index += 1;
      continue;
    }

    if (isTableLine(trimmed)) {
      const tableLines: string[] = [];
      while (index < lines.length && isTableLine(lines[index].trim())) {
        tableLines.push(lines[index].trim());
        index += 1;
      }
      blocks.push(renderTableBlock(tableLines));
      continue;
    }

    if (/^[-*]\s+/.test(trimmed)) {
      const items: string[] = [];
      while (index < lines.length && /^[-*]\s+/.test(lines[index].trim())) {
        items.push(lines[index].trim().replace(/^[-*]\s+/, ""));
        index += 1;
      }
      blocks.push(`<ul class="book-list">${items.map((item) => `<li>${renderInlineRichText(item)}</li>`).join("")}</ul>`);
      continue;
    }

    if (/^\d+\.\s+/.test(trimmed)) {
      const items: string[] = [];
      while (index < lines.length && /^\d+\.\s+/.test(lines[index].trim())) {
        items.push(lines[index].trim().replace(/^\d+\.\s+/, ""));
        index += 1;
      }
      blocks.push(`<ol class="book-list">${items.map((item) => `<li>${renderInlineRichText(item)}</li>`).join("")}</ol>`);
      continue;
    }

    if (/^>\s?/.test(trimmed)) {
      const quoteLines: string[] = [];
      while (index < lines.length && /^>\s?/.test(lines[index].trim())) {
        quoteLines.push(lines[index].trim().replace(/^>\s?/, ""));
        index += 1;
      }
      blocks.push(`<blockquote class="book-quote">${quoteLines.map((item) => `<p class="book-paragraph">${renderInlineRichText(item)}</p>`).join("")}</blockquote>`);
      continue;
    }

    const paragraphLines: string[] = [];
    while (
      index < lines.length
      && lines[index].trim()
      && !lines[index].trim().startsWith("```")
      && !isFigureLine(lines[index].trim())
      && !isTableLine(lines[index].trim())
      && !/^[-*]\s+/.test(lines[index].trim())
      && !/^\d+\.\s+/.test(lines[index].trim())
      && !/^>\s?/.test(lines[index].trim())
    ) {
      paragraphLines.push(lines[index].trim());
      index += 1;
    }
    blocks.push(`<p class="book-paragraph">${renderInlineRichText(paragraphLines.join(" "))}</p>`);
  }

  return blocks.join("");
}

export function renderInlineRichText(text: string) {
  return escapeHtml(text).replace(/`([^`]+)`/g, (_match, code) => `<code class="book-inline-code">${escapeHtml(code)}</code>`);
}

function renderFigureBlock(line: string) {
  const match = line.match(/^!\[(.*?)\]\((\S+?)(?:\s+"(.*)")?\)$/);
  if (!match) {
    return `<p class="book-paragraph">${renderInlineRichText(line)}</p>`;
  }
  const [, alt, src, title] = match;
  const caption = title || alt || "Figure";
  return `
    <figure class="book-figure">
      <img src="${escapeHtml(src)}" alt="${escapeHtml(alt || caption)}" />
      <figcaption>${escapeHtml(caption)}</figcaption>
    </figure>
  `;
}

function renderTableBlock(lines: string[]) {
  if (lines.length < 2) {
    return `<p class="book-paragraph">${renderInlineRichText(lines.join(" "))}</p>`;
  }

  const rows = lines
    .map((line) => line.trim().replace(/^\||\|$/g, "").split("|").map((cell) => cell.trim()))
    .filter((row) => row.length > 0);

  if (rows.length < 2) {
    return `<p class="book-paragraph">${renderInlineRichText(lines.join(" "))}</p>`;
  }

  const [header, ...body] = rows;
  const bodyRows = body.filter((row, index) => !(index === 0 && row.every((cell) => /^:?-{2,}:?$/.test(cell))));

  return `
    <figure class="book-table-wrap">
      <table class="book-table">
        <thead>
          <tr>${header.map((cell) => `<th>${renderInlineRichText(cell)}</th>`).join("")}</tr>
        </thead>
        <tbody>
          ${bodyRows.map((row) => `<tr>${row.map((cell) => `<td>${renderInlineRichText(cell)}</td>`).join("")}</tr>`).join("")}
        </tbody>
      </table>
      <figcaption>Technical table</figcaption>
    </figure>
  `;
}

function isFigureLine(line: string) {
  return /^!\[.*?\]\(\S+?(?:\s+".*?")?\)$/.test(line);
}

function isTableLine(line: string) {
  return line.startsWith("|") && line.endsWith("|");
}

export function summarizeText(text: string, maxLength = 220) {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (normalized.length <= maxLength) {
    return normalized;
  }
  return `${normalized.slice(0, maxLength - 1).trimEnd()}…`;
}

export function escapeHtml(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("\"", "&quot;")
    .replaceAll("'", "&#39;");
}
