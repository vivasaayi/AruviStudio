use crate::error::AppError;

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.to_string())
}

pub(crate) fn apply_unified_patch_text(original: &str, patch: &str) -> Result<String, AppError> {
    let cleaned_patch = strip_patch_fences(patch);
    let mut patch_lines = cleaned_patch.lines().peekable();
    while let Some(line) = patch_lines.peek().copied() {
        if line.starts_with("@@") {
            break;
        }
        patch_lines.next();
    }

    // A uniform CRLF file must remain CRLF after a patch. Mixed line endings
    // cannot be reconstructed reliably from unified diff lines.
    let crlf = original.contains("\r\n");
    if crlf && original.replace("\r\n", "").contains('\n') {
        return Err(invalid(
            "Cannot apply patch to a file with mixed line endings",
        ));
    }
    let source: Vec<&str> = original.lines().collect();
    let mut output: Vec<String> = Vec::new();
    let mut source_cursor = 0usize;
    let mut encountered_hunk = false;
    let mut old_no_newline = false;
    let mut new_no_newline_at: Option<usize> = None;
    let mut explicit_newline_change = false;

    while let Some(header) = patch_lines.next() {
        if !header.starts_with("@@") {
            return Err(invalid("Unexpected content between patch hunks"));
        }
        encountered_hunk = true;
        let ((old_start, old_count), (new_start, new_count)) = parse_hunk_header(header)?;
        let target = range_offset(old_start, old_count)?;
        if target < source_cursor || target > source.len() {
            return Err(invalid("Patch hunk position is invalid or overlapping"));
        }
        output.extend(
            source[source_cursor..target]
                .iter()
                .map(|line| (*line).to_string()),
        );
        source_cursor = target;
        if range_offset(new_start, new_count)? != output.len() {
            return Err(invalid("Patch new hunk position does not match output"));
        }
        let mut old_seen = 0usize;
        let mut new_seen = 0usize;
        let mut last_marker: Option<char> = None;
        while let Some(line) = patch_lines.peek().copied() {
            if line.starts_with("@@") {
                break;
            }
            patch_lines.next();
            if line == r"\ No newline at end of file" {
                let marker = last_marker
                    .take()
                    .ok_or_else(|| invalid("Misplaced no-newline marker"))?;
                if marker != '+' {
                    if source_cursor != source.len() || original.ends_with('\n') {
                        return Err(invalid("No-newline marker does not match source"));
                    }
                    old_no_newline = true;
                }
                if marker != '-' {
                    new_no_newline_at = Some(output.len());
                }
                if marker != ' ' {
                    explicit_newline_change = true;
                }
                continue;
            }
            let mut chars = line.chars();
            let marker = chars
                .next()
                .ok_or_else(|| invalid("Malformed patch line"))?;
            let value = chars.as_str();
            match marker {
                ' ' | '-' => {
                    old_seen = old_seen
                        .checked_add(1)
                        .ok_or_else(|| invalid("Patch count overflow"))?;
                    let source_line = source
                        .get(source_cursor)
                        .ok_or_else(|| invalid("Patch line exceeds source length"))?;
                    if *source_line != value {
                        return Err(invalid("Patch context or removal does not match source"));
                    }
                    source_cursor += 1;
                    if marker == ' ' {
                        new_seen = new_seen
                            .checked_add(1)
                            .ok_or_else(|| invalid("Patch count overflow"))?;
                        output.push(value.to_string());
                    }
                }
                '+' => {
                    new_seen = new_seen
                        .checked_add(1)
                        .ok_or_else(|| invalid("Patch count overflow"))?;
                    output.push(value.to_string());
                }
                _ => return Err(invalid("Unsupported patch marker")),
            }
            if old_seen > old_count || new_seen > new_count {
                return Err(invalid("Patch hunk exceeds declared line counts"));
            }
            last_marker = Some(marker);
        }
        if old_seen != old_count || new_seen != new_count {
            return Err(invalid("Patch hunk does not match declared line counts"));
        }
    }
    if !encountered_hunk {
        return Err(invalid("Patch did not contain any unified diff hunks"));
    }
    output.extend(
        source[source_cursor..]
            .iter()
            .map(|line| (*line).to_string()),
    );
    if let Some(position) = new_no_newline_at {
        if position != output.len() || position == 0 {
            return Err(invalid("No-newline marker is not on the final output line"));
        }
    }
    let mut rebuilt = output.join(if crlf { "\r\n" } else { "\n" });
    let trailing_newline = if new_no_newline_at.is_some() {
        false
    } else if explicit_newline_change && old_no_newline {
        true
    } else {
        original.ends_with('\n')
    };
    if trailing_newline && !output.is_empty() {
        rebuilt.push_str(if crlf { "\r\n" } else { "\n" });
    }
    Ok(rebuilt)
}

fn strip_patch_fences(patch: &str) -> String {
    let trimmed = patch.trim_matches(['\r', '\n']);
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    trimmed
        .lines()
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn range_offset(start: usize, count: usize) -> Result<usize, AppError> {
    if count == 0 {
        Ok(start)
    } else {
        start
            .checked_sub(1)
            .ok_or_else(|| invalid("Non-empty hunk range starts at zero"))
    }
}

fn parse_hunk_header(header: &str) -> Result<((usize, usize), (usize, usize)), AppError> {
    let remainder = header
        .strip_prefix("@@")
        .ok_or_else(|| invalid("Malformed hunk header"))?;
    let closing = remainder
        .find("@@")
        .ok_or_else(|| invalid("Malformed hunk header"))?;
    let mut parts = remainder[..closing].split_whitespace();
    let old = parts
        .next()
        .and_then(|part| part.strip_prefix('-'))
        .ok_or_else(|| invalid("Malformed old hunk range"))?;
    let new = parts
        .next()
        .and_then(|part| part.strip_prefix('+'))
        .ok_or_else(|| invalid("Malformed new hunk range"))?;
    if parts.next().is_some() {
        return Err(invalid("Malformed hunk header"));
    }
    Ok((parse_hunk_range(old)?, parse_hunk_range(new)?))
}

fn parse_hunk_range(range: &str) -> Result<(usize, usize), AppError> {
    let mut parts = range.split(',');
    let start = parts
        .next()
        .ok_or_else(|| invalid("Missing hunk start"))?
        .parse::<usize>()
        .map_err(|_| invalid("Invalid hunk start"))?;
    let count = parts
        .next()
        .map(str::parse::<usize>)
        .transpose()
        .map_err(|_| invalid("Invalid hunk count"))?
        .unwrap_or(1);
    if parts.next().is_some() {
        return Err(invalid("Malformed hunk range"));
    }
    Ok((start, count))
}

#[cfg(test)]
mod tests {
    use super::apply_unified_patch_text as apply;

    #[test]
    fn rejects_malformed_positions_and_counts() {
        for patch in [
            "@@ -999,0 +999,1 @@\n+x",
            "@@ -1,2 +1,1 @@\n-a\n+x",
            "@@ -1,1 +1,2 @@\n-a\n+x",
            "@@ -1,1 +2,1 @@\n-a\n+x",
            "@@ -1,1,1 +1,1 @@\n-a\n+x",
            "@@ -1,1 +1,1 @@\n-a\n+x\n--- another-file",
        ] {
            assert!(apply("a\nb\n", patch).is_err(), "{patch}");
        }
    }

    #[test]
    fn applies_zero_count_insertions_and_preserves_format() {
        assert_eq!(
            apply("a\nb\n", "@@ -0,0 +1,1 @@\n+start").unwrap(),
            "start\na\nb\n"
        );
        assert_eq!(
            apply("a\nb\n", "@@ -1,0 +2,1 @@\n+middle").unwrap(),
            "a\nmiddle\nb\n"
        );
        assert_eq!(
            apply("a\nb\n", "@@ -2,0 +3,1 @@\n+end  ").unwrap(),
            "a\nb\nend  \n"
        );
        assert_eq!(
            apply("a\r\nb\r\n", "@@ -1 +1 @@\n-a\n+x").unwrap(),
            "x\r\nb\r\n"
        );
        assert!(apply("a\r\nb\n", "@@ -1 +1 @@\n-a\n+x").is_err());
    }

    #[test]
    fn handles_explicit_no_newline_markers() {
        assert_eq!(
            apply(
                "old",
                "@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new"
            )
            .unwrap(),
            "new\n"
        );
        assert_eq!(
            apply(
                "old\n",
                "@@ -1 +1 @@\n-old\n+new\n\\ No newline at end of file"
            )
            .unwrap(),
            "new"
        );
        assert!(apply(
            "old\n",
            "@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new"
        )
        .is_err());
    }

    #[test]
    fn applies_multiple_hunks_and_rejects_overlaps() {
        let source = "a\nb\nc\nd\n";
        let patch = "@@ -1 +1 @@\n-a\n+A\n@@ -4 +4 @@\n-d\n+D";
        assert_eq!(apply(source, patch).unwrap(), "A\nb\nc\nD\n");
        let overlap = "@@ -1 +1 @@\n-a\n+A\n@@ -1 +1 @@\n-a\n+B";
        assert!(apply(source, overlap).is_err());
    }
}
