use crate::error::AppError;
use std::collections::HashMap;

pub(crate) struct CsvRecord {
    row_index: i64,
    fields: HashMap<String, String>,
}

pub(crate) fn parse_csv_records(content: &str) -> Result<Vec<CsvRecord>, AppError> {
    let rows = parse_csv_rows(content)?;
    let Some(headers) = rows.first() else {
        return Ok(Vec::new());
    };
    let headers = headers
        .iter()
        .map(|header| normalize_header(header))
        .collect::<Vec<_>>();
    let mut records = Vec::new();
    for (index, row) in rows.into_iter().enumerate().skip(1) {
        if row.iter().all(|value| value.trim().is_empty()) {
            continue;
        }
        let mut fields = HashMap::new();
        for (column_index, value) in row.into_iter().enumerate() {
            let Some(header) = headers.get(column_index) else {
                continue;
            };
            if header.is_empty() {
                continue;
            }
            let value = value.trim().to_string();
            if !value.is_empty() {
                fields.insert(header.clone(), value);
            }
        }
        records.push(CsvRecord {
            row_index: i64::try_from(index + 1).unwrap_or(i64::MAX),
            fields,
        });
    }
    Ok(records)
}

pub(crate) fn csv_field(record: &CsvRecord, keys: &[&str]) -> Option<String> {
    for key in keys {
        let normalized = normalize_header(key);
        if let Some(value) = record.fields.get(&normalized) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

pub(crate) fn csv_list_field(record: &CsvRecord, keys: &[&str]) -> Vec<String> {
    csv_field(record, keys)
        .map(|value| {
            value
                .split([';', '|'])
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn csv_error(record: &CsvRecord, message: &str) -> AppError {
    AppError::Validation(format!("CSV row {}: {message}", record.row_index))
}

pub(crate) fn normalize_record_type(value: &str) -> String {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "productarea" => "product_area".to_string(),
        "story" => "story".to_string(),
        "task" => "task".to_string(),
        other => other.to_string(),
    }
}

fn parse_csv_rows(content: &str) -> Result<Vec<Vec<String>>, AppError> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = content.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                row.push(std::mem::take(&mut field));
            }
            '\n' if !in_quotes => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            '\r' if !in_quotes => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            _ => field.push(ch),
        }
    }

    if in_quotes {
        return Err(AppError::Validation(
            "CSV import has an unterminated quoted field.".to_string(),
        ));
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

fn normalize_header(value: &str) -> String {
    let mut normalized = String::new();
    for ch in value.trim().trim_start_matches('\u{feff}').chars() {
        if ch == '-' || ch == ' ' {
            if !normalized.ends_with('_') {
                normalized.push('_');
            }
        } else if ch.is_ascii_uppercase() {
            if !normalized.is_empty() && !normalized.ends_with('_') {
                normalized.push('_');
            }
            normalized.push(ch.to_ascii_lowercase());
        } else {
            normalized.push(ch.to_ascii_lowercase());
        }
    }
    normalized
}
