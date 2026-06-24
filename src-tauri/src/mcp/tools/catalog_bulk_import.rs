use crate::error::AppError;
use crate::services::bulk_import_service::{self, BulkImportRequest};
use crate::state::AppState;
use serde_json::Value;

use super::action_args::ActionArgs;
use super::action_result;

pub(super) async fn handle(
    state: &AppState,
    action: &str,
    args: &ActionArgs<'_>,
) -> Result<Value, AppError> {
    match action {
        "get_bulk_import_schema" => action_result(
            "get_bulk_import_schema",
            bulk_import_service::bulk_import_schema(),
        ),
        "submit_bulk_import" => {
            let file_path = args.required_string(&["file_path", "filePath"], "file_path")?;
            let job = bulk_import_service::submit_bulk_import(
                (*state).clone(),
                BulkImportRequest {
                    file_path,
                    format: args.optional_string(&["format"])?,
                    product_id: args.optional_string(&["product_id", "productId"])?,
                },
            )
            .await?;
            action_result("submit_bulk_import", job)
        }
        "get_bulk_import_status" => {
            let job_id = args.required_string(&["job_id", "jobId"], "job_id")?;
            action_result(
                "get_bulk_import_status",
                bulk_import_service::get_bulk_import_status(&state.db, &job_id).await?,
            )
        }
        "list_bulk_import_jobs" => action_result(
            "list_bulk_import_jobs",
            bulk_import_service::list_bulk_import_jobs(&state.db, args.optional_i64(&["limit"])?)
                .await?,
        ),
        _ => Err(AppError::Validation(format!(
            "unsupported bulk import catalog action: {action}"
        ))),
    }
}
