use crate::error::AppError;

pub(crate) fn normalize_status(status: &str) -> Result<&'static str, AppError> {
    match status.trim().to_ascii_lowercase().as_str() {
        "pending" => Ok("pending"),
        "claimed" => Ok("claimed"),
        "in_progress" | "in-progress" => Ok("in_progress"),
        "implemented" => Ok("implemented"),
        "tests_passed" | "tests-passed" => Ok("tests_passed"),
        "committed" => Ok("committed"),
        "blocked" => Ok("blocked"),
        "skipped" => Ok("skipped"),
        "cancelled" | "canceled" => Ok("cancelled"),
        other => Err(AppError::Validation(format!(
            "Unsupported agent work status '{other}'."
        ))),
    }
}

pub(crate) fn normalize_batch_status(status: &str) -> Result<&'static str, AppError> {
    match status.trim().to_ascii_lowercase().as_str() {
        "claimed" => Ok("claimed"),
        "in_progress" | "in-progress" => Ok("in_progress"),
        "implemented" => Ok("implemented"),
        "tests_passed" | "tests-passed" => Ok("tests_passed"),
        "committed" => Ok("committed"),
        "blocked" => Ok("blocked"),
        "skipped" => Ok("skipped"),
        "cancelled" | "canceled" => Ok("cancelled"),
        other => Err(AppError::Validation(format!(
            "Unsupported agent work batch status '{other}'."
        ))),
    }
}
