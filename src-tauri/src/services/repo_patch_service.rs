use crate::error::AppError;

pub(crate) fn apply_unified_patch_text(original: &str, patch: &str) -> Result<String, AppError> {
    let cleaned_patch = strip_patch_fences(patch);
    let mut patch_lines = cleaned_patch.lines().peekable();
    while let Some(line) = patch_lines.peek().copied() {
        if line.starts_with("@@") {
            break;
        }
        patch_lines.next();
    }

    let source = original
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    let original_had_trailing_newline = original.ends_with('\n');
    let mut output: Vec<String> = Vec::new();
    let mut source_cursor: usize = 0;
    let mut encountered_hunk = false;

    while let Some(line) = patch_lines.next() {
        if !line.starts_with("@@") {
            continue;
        }
        encountered_hunk = true;
        let (old_start, _old_count) = parse_hunk_header(line)?;
        let target_cursor = old_start.saturating_sub(1);
        if target_cursor < source_cursor {
            return Err(AppError::Validation(
                "Patch hunk order is invalid or overlapping".to_string(),
            ));
        }

        output.extend_from_slice(&source[source_cursor..target_cursor.min(source.len())]);
        source_cursor = target_cursor.min(source.len());

        while let Some(next_line) = patch_lines.peek().copied() {
            if next_line.starts_with("@@") {
                break;
            }
            patch_lines.next();
            if next_line == r"\ No newline at end of file" {
                continue;
            }
            let mut chars = next_line.chars();
            let marker = chars
                .next()
                .ok_or_else(|| AppError::Validation("Malformed patch line".to_string()))?;
            let value = chars.as_str().to_string();
            match marker {
                ' ' => {
                    let source_line = source.get(source_cursor).ok_or_else(|| {
                        AppError::Validation("Patch context line exceeds source length".to_string())
                    })?;
                    if source_line != &value {
                        return Err(AppError::Validation(format!(
                            "Patch context mismatch. Expected '{}', found '{}'",
                            value, source_line
                        )));
                    }
                    output.push(source_line.clone());
                    source_cursor += 1;
                }
                '-' => {
                    let source_line = source.get(source_cursor).ok_or_else(|| {
                        AppError::Validation("Patch removal line exceeds source length".to_string())
                    })?;
                    if source_line != &value {
                        return Err(AppError::Validation(format!(
                            "Patch removal mismatch. Expected '{}', found '{}'",
                            value, source_line
                        )));
                    }
                    source_cursor += 1;
                }
                '+' => {
                    output.push(value);
                }
                _ => {
                    return Err(AppError::Validation(format!(
                        "Unsupported patch marker '{}'",
                        marker
                    )));
                }
            }
        }
    }

    if !encountered_hunk {
        return Err(AppError::Validation(
            "Patch did not contain any unified diff hunks".to_string(),
        ));
    }

    output.extend_from_slice(&source[source_cursor..]);
    let mut rebuilt = output.join("\n");
    if original_had_trailing_newline && !rebuilt.ends_with('\n') {
        rebuilt.push('\n');
    }
    Ok(rebuilt)
}

fn strip_patch_fences(patch: &str) -> String {
    let trimmed = patch.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    let mut lines = trimmed.lines();
    let first = lines.next().unwrap_or_default();
    if !first.starts_with("```") {
        return trimmed.to_string();
    }
    let mut body: Vec<String> = Vec::new();
    for line in lines {
        if line.trim_start().starts_with("```") {
            break;
        }
        body.push(line.to_string());
    }
    body.join("\n")
}

fn parse_hunk_header(header: &str) -> Result<(usize, usize), AppError> {
    // @@ -old_start,old_count +new_start,new_count @@
    let remainder = header
        .strip_prefix("@@")
        .ok_or_else(|| AppError::Validation("Malformed hunk header".to_string()))?;
    let closing = remainder
        .find("@@")
        .ok_or_else(|| AppError::Validation("Malformed hunk header".to_string()))?;
    let inner = remainder[..closing].trim();
    let mut parts = inner.split_whitespace();
    let old_part = parts
        .next()
        .ok_or_else(|| AppError::Validation("Malformed old range in hunk header".to_string()))?;
    if !old_part.starts_with('-') {
        return Err(AppError::Validation(
            "Malformed old range in hunk header".to_string(),
        ));
    }
    parse_hunk_range(&old_part[1..])
}

fn parse_hunk_range(range: &str) -> Result<(usize, usize), AppError> {
    let mut parts = range.split(',');
    let start = parts
        .next()
        .ok_or_else(|| AppError::Validation("Missing hunk start".to_string()))?
        .parse::<usize>()
        .map_err(|_| AppError::Validation("Invalid hunk start".to_string()))?;
    let count = parts
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| AppError::Validation("Invalid hunk count".to_string()))?
        .unwrap_or(1);
    Ok((start, count))
}
