use super::*;
use crate::persistence::{product_repo, work_item_repo};
use serde_json::json;

#[tokio::test]
async fn submit_planner_voice_turn_selects_draft_nodes() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("voice_selects_draft_node").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None)
        .await
        .expect("failed to create planner session");

    let draft = apply_actions_to_draft(
        None,
        None,
        &normalize_actions(vec![
            json!({
                "type": "create_product",
                "name": "Hotel Management System",
                "description": "Hotel root."
            }),
            json!({
                "type": "create_product_area",
                "target": { "productName": "Hotel Management System" },
                "name": "Guest Management",
                "description": "Guest workflows."
            }),
        ]),
    )
    .expect("failed to build draft");
    let product_id = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product")
        .expect("missing product")
        .id
        .clone();
    let product_area_id = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product_area" && node.name == "Guest Management")
        .expect("missing product_area")
        .id
        .clone();

    {
        let mut service = state.planner_service.lock().await;
        let mut loaded = service
            .get_session(&session.session_id)
            .expect("planner session should exist");
        loaded.draft_plan = Some(draft.clone());
        loaded.selected_draft_node_id = Some(product_id.clone());
        service.save_session(&session.session_id, loaded);
    }
    persist_draft_state(
        &state.db,
        &session.session_id,
        Some(&draft),
        Some(product_id.as_str()),
    )
    .await
    .expect("failed to persist draft state");

    let response = submit_planner_voice_turn(
        state.planner_service.clone(),
        &state,
        session.session_id.clone(),
        "select product area guest management".to_string(),
        None,
        None,
    )
    .await
    .expect("voice turn should succeed");

    assert_eq!(response.status, "session_update");
    assert_eq!(
        response.selected_draft_node_id.as_deref(),
        Some(product_area_id.as_str())
    );
    assert!(response
        .assistant_message
        .contains("Selected product area \"Guest Management\""));
    assert!(response.draft_tree_nodes.is_some());
}

#[tokio::test]
async fn voice_view_commands_are_handled_without_a_model() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("voice_view_commands").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None)
        .await
        .expect("create session");
    for (command, view) in [("Show trace.", "trace"), ("Back to chat.", "conversation")] {
        let response = submit_planner_voice_turn(
            state.planner_service.clone(),
            &state,
            session.session_id.clone(),
            command.into(),
            None,
            None,
        )
        .await
        .expect("view command does not need a provider");
        assert_eq!(response.status, "session_update");
        assert_eq!(response.view_hint.as_deref(), Some(view));
        assert!(response.pending_plan.is_none());
    }
}

#[tokio::test]
async fn commit_draft_plan_persists_tree_structure() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("commit_draft").await;

    let actions = normalize_actions(vec![
        json!({
            "type": "create_product",
            "target": "Hotel Management System",
            "name": "Hotel Management System",
            "description": "Hotel root."
        }),
        json!({
            "type": "create_product_area",
            "target": "Hotel Management System",
            "product_area_name": "Guest Management",
            "description": "Guest workflows."
        }),
        json!({
            "type": "create_capability",
            "target": "Guest Management",
            "capability_name": "Guest Profile Management",
            "description": "Profiles."
        }),
        json!({
            "type": "create_work_item",
            "target": "Guest Profile Management",
            "work_item_name": "Implement Guest Profile CRUD",
            "description": "Build CRUD."
        }),
    ]);
    let draft: PlannerDraftPlan =
        apply_actions_to_draft(None, None, &actions).expect("failed to build draft");

    let execution = commit_draft_plan(&state, &draft)
        .await
        .expect("failed to commit draft");
    assert!(!execution.is_empty());

    let products = product_repo::list_products(&state.db)
        .await
        .expect("failed to list products");
    let product = products
        .iter()
        .find(|product| product.name == "Hotel Management System")
        .expect("committed product not found");

    let product_areas = product_repo::list_product_areas(&state.db, &product.id)
        .await
        .expect("failed to list product_areas");
    let product_area = product_areas
        .iter()
        .find(|product_area| product_area.name == "Guest Management")
        .expect("committed product_area not found");

    let tree = product_repo::get_product_tree(&state.db, &product.id)
        .await
        .expect("failed to load product tree");
    let capability = tree
        .product_areas
        .iter()
        .find(|product_area_tree| product_area_tree.product_area.id == product_area.id)
        .and_then(|product_area_tree| product_area_tree.features.first())
        .map(|feature| feature.capability.name.clone());
    assert_eq!(capability.as_deref(), Some("Guest Profile Management"));

    let work_items = work_item_repo::list_work_items_page(
        &state.db,
        work_item_repo::WorkItemListQuery {
            product_id: Some(&product.id),
            limit: Some(20),
            offset: Some(0),
            ..Default::default()
        },
    )
    .await
    .expect("failed to list work items");
    assert!(work_items
        .iter()
        .any(|item| item.title == "Implement Guest Profile CRUD"));
}

#[tokio::test]
async fn concurrent_draft_edits_are_serialized_per_session() {
    use crate::services::planner_draft_commands::add_planner_draft_child;
    use crate::services::planner_session::{load_session_from_db, lock_planner_session};
    use std::time::Duration;

    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("concurrent_draft_edits").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None)
        .await
        .unwrap();
    let draft = apply_actions_to_draft(
        None,
        None,
        &normalize_actions(vec![json!({
            "type": "create_product", "name": "Concurrent Product", "description": "Test root"
        })]),
    )
    .unwrap();
    let root_id = draft
        .nodes
        .iter()
        .find(|node| node.node_type == "product")
        .unwrap()
        .id
        .clone();
    {
        let mut service = state.planner_service.lock().await;
        let mut current = service.get_session(&session.session_id).unwrap();
        current.draft_plan = Some(draft.clone());
        service.save_session(&session.session_id, current);
    }
    persist_draft_state(&state.db, &session.session_id, Some(&draft), None)
        .await
        .unwrap();

    let second_session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    let held = lock_planner_session(&state.planner_service, &state.db, &session.session_id).await.unwrap();
    let other = tokio::time::timeout(
        Duration::from_secs(1),
        lock_planner_session(&state.planner_service, &state.db, &second_session.session_id),
    )
    .await
    .expect("another session should not be blocked").unwrap();
    drop(other);
    let mut edits = Vec::new();
    for name in ["First Area", "Second Area"] {
        let service = state.planner_service.clone();
        let db = state.db.clone();
        let id = session.session_id.clone();
        let root = root_id.clone();
        edits.push(tokio::spawn(async move {
            add_planner_draft_child(
                service,
                &db,
                id,
                root,
                "product_area".into(),
                name.into(),
                None,
            )
            .await
        }));
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        edits.iter().all(|edit| !edit.is_finished()),
        "edits must wait for the session lock"
    );
    drop(held);
    for edit in edits {
        edit.await.unwrap().unwrap();
    }
    let persisted = load_session_from_db(&state.db, &session.session_id)
        .await
        .unwrap();
    let draft = persisted.draft_plan.unwrap();
    for name in ["First Area", "Second Area"] {
        assert!(
            draft.nodes.iter().any(|node| node.name == name),
            "missing {name}"
        );
    }
}

#[tokio::test]
async fn explicit_planner_model_never_falls_back_to_another_provider() {
    use crate::persistence::model_repo;
    use crate::services::planner_model::resolve_planner_model_binding;

    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("explicit_planner_binding").await;
    model_repo::create_provider(
        &state.db,
        "planner-test-provider",
        "Planner Test",
        "openai_compatible",
        "http://localhost:12345/v1",
        None,
    )
    .await
    .unwrap();
    model_repo::create_model_definition(
        &state.db,
        "planner-test-model",
        "planner-test-provider",
        "planner-test-name",
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let resolve = |provider: Option<&str>, model: Option<&str>| {
        resolve_planner_model_binding(
            &state.db,
            provider.map(str::to_owned),
            model.map(str::to_owned),
        )
    };
    assert_eq!(
        resolve(Some("planner-test-provider"), Some("planner-test-name"))
            .await
            .unwrap(),
        (
            Some("planner-test-provider".into()),
            Some("planner-test-name".into())
        )
    );
    assert!(
        resolve(Some("planner-test-provider"), Some("missing-model"))
            .await
            .is_err()
    );
    assert!(resolve(Some("missing-provider"), Some("planner-test-name"))
        .await
        .is_err());
    assert!(resolve(Some("missing-provider"), None).await.is_err());
    assert!(resolve(None, Some("missing-model")).await.is_err());
    assert_eq!(
        resolve(Some("planner-test-provider"), None).await.unwrap(),
        (
            Some("planner-test-provider".into()),
            Some("planner-test-name".into())
        )
    );
    model_repo::create_model_definition(&state.db, "planner-voice-only", "planner-test-provider", "deepseek-voice-transcription", None, None, None).await.unwrap();
    assert!(resolve(Some("planner-test-provider"), Some("deepseek-voice-transcription")).await.is_err());
    assert_eq!(resolve(Some("planner-test-provider"), None).await.unwrap(), (Some("planner-test-provider".into()), Some("planner-test-name".into())));
    model_repo::create_provider(&state.db, "planner-local-speech", "Local speech", "local_runtime", "/tmp/fixture-whisper.bin", None).await.unwrap();
    model_repo::create_model_definition(&state.db, "planner-local-model", "planner-local-speech", "whisper-fixture", None, None, None).await.unwrap();
    assert!(resolve(Some("planner-local-speech"), Some("whisper-fixture")).await.is_err());
}

#[tokio::test]
async fn commit_draft_plan_rolls_back_catalog_on_late_failure() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("commit_draft_rollback").await;
    let original = product_repo::create_product(
        &state.db,
        product_repo::CreateProductInput {
            id: "rollback-product",
            name: "Rollback Product",
            description: "Original description",
            vision: "", goals: "", tags: "", lifecycle: None, health: None,
            owner_label: None, investment_status: None, roadmap: None, evidence: None,
        },
    ).await.expect("create original product");
    let actions = normalize_actions(vec![
        json!({"type":"create_product", "name":"Rollback Product", "description":"Changed description"}),
        json!({"type":"create_product_area", "target":"Rollback Product", "product_area_name":"Area"}),
        json!({"type":"create_capability", "target":"Area", "capability_name":"Capability"}),
    ]);
    let mut draft = apply_actions_to_draft(None, None, &actions).expect("build draft");
    draft.nodes.iter_mut().find(|node| node.node_type == "capability")
        .expect("capability").parent_id = Some("missing-parent".to_string());
    assert!(commit_draft_plan(&state, &draft).await.is_err());
    let product = product_repo::get_product(&state.db, &original.id).await.expect("load product");
    assert_eq!(product.description, "Original description");
    assert!(product_repo::list_product_areas(&state.db, &original.id).await.expect("list areas").is_empty());
}

#[tokio::test]
async fn confirmation_rolls_back_catalog_if_conversation_write_fails() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("confirmation_rollback").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None)
        .await.expect("create session");
    let actions = normalize_actions(vec![json!({"type":"create_product", "name":"Atomic Product"})]);
    let draft = apply_actions_to_draft(None, None, &actions).expect("build draft");
    {
        let mut service = state.planner_service.lock().await;
        let mut loaded = service.get_session(&session.session_id).expect("session");
        loaded.draft_plan = Some(draft.clone());
        service.save_session(&session.session_id, loaded);
    }
    persist_draft_state(&state.db, &session.session_id, Some(&draft), None).await.expect("persist draft");
    sqlx::query("CREATE TRIGGER fail_assistant_entry BEFORE INSERT ON planner_conversation_entries WHEN NEW.role = 'assistant' BEGIN SELECT RAISE(ABORT, 'injected failure'); END")
        .execute(&state.db).await.expect("create trigger");
    let result = crate::services::planner_text_turn::confirm_planner_plan(
        state.planner_service.clone(), &state, session.session_id.clone(),
    ).await;
    assert!(result.is_err());
    assert!(product_repo::list_products(&state.db).await.expect("list products").iter().all(|p| p.name != "Atomic Product"));
    let record = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.expect("load session");
    assert!(record.draft_plan_json.is_some());
    assert!(crate::persistence::planner_repo::list_conversation_entries(&state.db, &session.session_id).await.expect("list conversation").is_empty());
    assert!(state.planner_service.lock().await.get_session(&session.session_id).expect("cached session").draft_plan.is_some());
}

#[tokio::test]
async fn persisted_session_can_be_updated_and_cleared_after_service_restart() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("session_after_restart").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None)
        .await.expect("create session");
    let draft = apply_actions_to_draft(None, None, &normalize_actions(vec![
        json!({"type":"create_product", "name":"Restart fixture"}),
    ])).expect("build draft");
    persist_draft_state(&state.db, &session.session_id, Some(&draft), None).await.expect("persist draft");
    let restarted = Arc::new(Mutex::new(crate::services::planner_service::PlannerService::new()));
    let info = crate::services::planner_service::update_planner_session(
        restarted.clone(), &state.db, session.session_id.clone(), None, None,
    ).await.expect("update persisted session after restart");
    assert!(info.has_draft_plan);
    let info = crate::services::planner_service::clear_planner_pending(
        restarted, &state.db, session.session_id.clone(),
    ).await.expect("clear persisted session after restart");
    assert!(!info.has_draft_plan && !info.has_pending_plan);
    let record = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.expect("load session");
    assert!(record.draft_plan_json.is_none() && record.pending_plan_json.is_none());
}

#[tokio::test]
async fn failed_session_clear_keeps_persisted_and_cached_draft() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("session_clear_failure").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None)
        .await.expect("create session");
    let draft = apply_actions_to_draft(None, None, &normalize_actions(vec![
        json!({"type":"create_product", "name":"Failure fixture"}),
    ])).expect("build draft");
    persist_draft_state(&state.db, &session.session_id, Some(&draft), None).await.expect("persist draft");
    let pending = json!({"assistant_response":"Review", "needs_confirmation":true, "clarification_question":null, "actions":[]}).to_string();
    crate::persistence::planner_repo::update_pending_plan(&state.db, &session.session_id, Some(&pending)).await.unwrap();
    let restarted = Arc::new(Mutex::new(crate::services::planner_service::PlannerService::new()));
    sqlx::query("CREATE TRIGGER fail_clear BEFORE UPDATE OF draft_plan_json ON planner_sessions BEGIN SELECT RAISE(ABORT, 'injected failure'); END")
        .execute(&state.db).await.expect("create trigger");
    assert!(crate::services::planner_service::clear_planner_pending(
        restarted.clone(), &state.db, session.session_id.clone(),
    ).await.is_err());
    let persisted = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.unwrap();
    assert!(persisted.draft_plan_json.is_some() && persisted.pending_plan_json.is_some());
    let cached = restarted.lock().await.get_session(&session.session_id).unwrap();
    assert!(cached.draft_plan.is_some() && cached.pending_plan.is_some());
}

#[tokio::test]
async fn failed_session_update_keeps_cached_settings() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("session_update_failure").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None)
        .await.expect("create session");
    crate::persistence::model_repo::create_provider(&state.db, "new-provider", "New provider", "openai_compatible", "http://localhost:12345", None).await.unwrap();
    crate::persistence::model_repo::create_model_definition(&state.db, "new-model-id", "new-provider", "new-model", None, None, None).await.unwrap();
    let restarted = Arc::new(Mutex::new(crate::services::planner_service::PlannerService::new()));
    sqlx::query("CREATE TRIGGER fail_settings BEFORE UPDATE OF provider_id ON planner_sessions BEGIN SELECT RAISE(ABORT, 'injected failure'); END")
        .execute(&state.db).await.expect("create trigger");
    assert!(crate::services::planner_service::update_planner_session(
        restarted.clone(), &state.db, session.session_id.clone(), Some("new-provider".into()), Some("new-model".into()),
    ).await.is_err());
    let persisted = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.unwrap();
    let cached = restarted.lock().await.get_session(&session.session_id).unwrap();
    assert_eq!(cached.provider_id, persisted.provider_id);
    assert_eq!(cached.model_name, persisted.model_name);
}

#[cfg(unix)]
#[tokio::test]
async fn separate_services_preserve_concurrent_session_edits() {
    use crate::services::planner_draft_commands::add_planner_draft_child;
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("independent_planner_services").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    let draft = apply_actions_to_draft(None, None, &normalize_actions(vec![
        json!({"type":"create_product", "name":"Shared product"}),
    ])).unwrap();
    let root = draft.nodes.iter().find(|node| node.node_type == "product").unwrap().id.clone();
    persist_draft_state(&state.db, &session.session_id, Some(&draft), None).await.unwrap();
    let other = Arc::new(Mutex::new(crate::services::planner_service::PlannerService::new()));
    // Prime both caches before either mutation to reproduce the stale-cache case.
    crate::services::planner_session::get_or_load_session(&state.planner_service, &state.db, &session.session_id).await.unwrap();
    crate::services::planner_session::get_or_load_session(&other, &state.db, &session.session_id).await.unwrap();
    let mut tasks = Vec::new();
    for (service, name) in [(state.planner_service.clone(), "First"), (other, "Second")] {
        let db = state.db.clone();
        let session_id = session.session_id.clone();
        let root = root.clone();
        tasks.push(tokio::spawn(async move {
            add_planner_draft_child(service, &db, session_id, root, "product_area".into(), name.into(), None).await
        }));
    }
    for task in tasks { task.await.unwrap().unwrap(); }
    let persisted = crate::services::planner_session::load_session_from_db(&state.db, &session.session_id).await.unwrap();
    let names = persisted.draft_plan.unwrap().nodes.into_iter().map(|node| node.name).collect::<Vec<_>>();
    assert!(names.iter().any(|name| name == "First"));
    assert!(names.iter().any(|name| name == "Second"));
}

#[cfg(unix)]
#[tokio::test]
async fn planner_lock_child_process() {
    let Ok(url) = std::env::var("ARUVI_PLANNER_LOCK_CHILD_DB") else { return; };
    let session_id = std::env::var("ARUVI_PLANNER_LOCK_CHILD_SESSION").unwrap();
    let marker = std::env::var("ARUVI_PLANNER_LOCK_CHILD_MARKER").unwrap();
    let pool = crate::persistence::db::create_pool(&url).await.unwrap();
    let service = Arc::new(Mutex::new(crate::services::planner_service::PlannerService::new()));
    std::fs::write(std::path::Path::new(&marker).with_extension("ready"), "ready").unwrap();
    let _guard = crate::services::planner_session::lock_planner_session(&service, &pool, &session_id).await.unwrap();
    std::fs::write(marker, "acquired").unwrap();
    if std::env::var_os("ARUVI_PLANNER_LOCK_CHILD_HOLD").is_some() {
        std::future::pending::<()>().await;
    }
}

#[cfg(unix)]
#[tokio::test]
async fn planner_session_lock_coordinates_separate_processes() {
    use std::time::{Duration, Instant};
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("planner_process_lock").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    let held = crate::services::planner_session::lock_planner_session(&state.planner_service, &state.db, &session.session_id).await.unwrap();
    let dbs: Vec<(i64, String, String)> = sqlx::query_as("PRAGMA database_list").fetch_all(&state.db).await.unwrap();
    let path = &dbs.iter().find(|(_, name, _)| name == "main").unwrap().2;
    let marker = std::path::Path::new(path).with_extension("child-acquired");
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "services::planner_service::tests::planner_turns::planner_lock_child_process", "--nocapture"])
        .env("ARUVI_PLANNER_LOCK_CHILD_DB", format!("sqlite:{path}"))
        .env("ARUVI_PLANNER_LOCK_CHILD_SESSION", &session.session_id)
        .env("ARUVI_PLANNER_LOCK_CHILD_MARKER", &marker)
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
    let ready = marker.with_extension("ready");
    let startup_deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() {
        if let Some(status) = child.try_wait().unwrap() { panic!("child exited before attempting lock: {status}"); }
        if Instant::now() >= startup_deadline { let _ = child.kill(); let _ = child.wait(); panic!("child startup timed out"); }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(child.try_wait().unwrap().is_none(), "child exited before lock was released");
    assert!(!marker.exists(), "child acquired a held lock");
    drop(held);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() { assert!(status.success(), "child lock test failed"); break; }
        if Instant::now() >= deadline { let _ = child.kill(); let _ = child.wait(); panic!("child lock test timed out"); }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(marker.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn cancelling_waiting_planner_lock_does_not_block_later_requests() {
    use std::time::Duration;
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("planner_lock_cancellation").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    let held = crate::services::planner_session::lock_planner_session(&state.planner_service, &state.db, &session.session_id).await.unwrap();
    let other = Arc::new(Mutex::new(crate::services::planner_service::PlannerService::new()));
    let db = state.db.clone();
    let session_id = session.session_id.clone();
    let waiting = tokio::spawn(async move {
        crate::services::planner_session::lock_planner_session(&other, &db, &session_id).await
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!waiting.is_finished());
    waiting.abort();
    match waiting.await { Err(error) => assert!(error.is_cancelled()), Ok(_) => panic!("cancelled lock unexpectedly completed") };
    drop(held);
    let other = Arc::new(Mutex::new(crate::services::planner_service::PlannerService::new()));
    let acquired = tokio::time::timeout(Duration::from_secs(2), crate::services::planner_session::lock_planner_session(&other, &state.db, &session.session_id)).await.unwrap().unwrap();
    drop(acquired);
}

#[tokio::test]
async fn reloaded_conversation_preserves_order_for_equal_timestamps() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("planner_conversation_order").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    for (id, role, content) in [("z", "user", "first"), ("a", "assistant", "second"), ("m", "user", "third")] {
        sqlx::query("INSERT INTO planner_conversation_entries (id, session_id, role, content, created_at) VALUES (?, ?, ?, ?, '2026-01-01 00:00:00')")
            .bind(id).bind(&session.session_id).bind(role).bind(content).execute(&state.db).await.unwrap();
    }
    let reloaded = crate::services::planner_session::load_session_from_db(&state.db, &session.session_id).await.unwrap();
    let content = reloaded.conversation.iter().map(|entry| entry.content.as_str()).collect::<Vec<_>>();
    assert_eq!(content, ["first", "second", "third"]);
}

#[cfg(unix)]
#[tokio::test]
async fn planner_rejects_symlinked_lock_directory() {
    use std::os::unix::fs::symlink;
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("planner_lock_symlink").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    let dbs: Vec<(i64, String, String)> = sqlx::query_as("PRAGMA database_list").fetch_all(&state.db).await.unwrap();
    let path = std::fs::canonicalize(&dbs.iter().find(|(_, name, _)| name == "main").unwrap().2).unwrap();
    let mut directory_name = std::ffi::OsString::from(".");
    directory_name.push(path.file_name().unwrap());
    directory_name.push(".planner-locks");
    let target = path.parent().unwrap().join("unrelated-dir");
    std::fs::create_dir(&target).unwrap();
    symlink(&target, path.parent().unwrap().join(directory_name)).unwrap();
    assert!(crate::services::planner_session::lock_planner_session(&state.planner_service, &state.db, &session.session_id).await.is_err());
    assert!(std::fs::read_dir(target).unwrap().next().is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn planner_session_lock_is_released_when_process_is_killed() {
    use std::time::{Duration, Instant};
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("planner_process_crash").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    let dbs: Vec<(i64, String, String)> = sqlx::query_as("PRAGMA database_list").fetch_all(&state.db).await.unwrap();
    let path = &dbs.iter().find(|(_, name, _)| name == "main").unwrap().2;
    let marker = std::path::Path::new(path).with_extension("child-held");
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "services::planner_service::tests::planner_turns::planner_lock_child_process", "--nocapture"])
        .env("ARUVI_PLANNER_LOCK_CHILD_DB", format!("sqlite:{path}"))
        .env("ARUVI_PLANNER_LOCK_CHILD_SESSION", &session.session_id)
        .env("ARUVI_PLANNER_LOCK_CHILD_MARKER", &marker)
        .env("ARUVI_PLANNER_LOCK_CHILD_HOLD", "1")
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !marker.exists() {
        if let Some(status) = child.try_wait().unwrap() { panic!("child exited before taking lock: {status}"); }
        if Instant::now() >= deadline { let _ = child.kill(); let _ = child.wait(); panic!("child lock timed out"); }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let blocked = tokio::time::timeout(Duration::from_millis(200), crate::services::planner_session::lock_planner_session(&state.planner_service, &state.db, &session.session_id)).await;
    let _ = child.kill();
    let _ = child.wait();
    assert!(blocked.is_err(), "lock should remain held by child");
    let acquired = tokio::time::timeout(Duration::from_secs(2), crate::services::planner_session::lock_planner_session(&state.planner_service, &state.db, &session.session_id)).await.unwrap().unwrap();
    drop(acquired);
}

#[tokio::test]
async fn failed_manual_draft_edits_preserve_pending_plan_and_draft() {
    use crate::services::planner_draft_commands::{add_planner_draft_child, delete_planner_draft_node, rename_planner_draft_node};
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("manual_draft_write_failure").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    let draft = apply_actions_to_draft(None, None, &normalize_actions(vec![
        json!({"type":"create_product", "name":"Draft root"}),
        json!({"type":"create_product_area", "target":"Draft root", "product_area_name":"Draft area"}),
    ])).unwrap();
    let root = draft.nodes.iter().find(|node| node.node_type == "product").unwrap().id.clone();
    let area = draft.nodes.iter().find(|node| node.node_type == "product_area").unwrap().id.clone();
    persist_draft_state(&state.db, &session.session_id, Some(&draft), Some(&root)).await.unwrap();
    let pending = json!({"assistant_response":"Review", "needs_confirmation":true, "clarification_question":null, "actions":[]}).to_string();
    crate::persistence::planner_repo::update_pending_plan(&state.db, &session.session_id, Some(&pending)).await.unwrap();
    let before = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_draft_edit BEFORE UPDATE OF draft_plan_json ON planner_sessions BEGIN SELECT RAISE(ABORT, 'injected failure'); END")
        .execute(&state.db).await.unwrap();
    assert!(rename_planner_draft_node(state.planner_service.clone(), &state.db, session.session_id.clone(), area.clone(), "Renamed".into()).await.is_err());
    assert!(add_planner_draft_child(state.planner_service.clone(), &state.db, session.session_id.clone(), root.clone(), "product_area".into(), "Added".into(), None).await.is_err());
    assert!(delete_planner_draft_node(state.planner_service.clone(), &state.db, session.session_id.clone(), area).await.is_err());
    let after = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.unwrap();
    assert_eq!(after.pending_plan_json, before.pending_plan_json);
    assert_eq!(after.draft_plan_json, before.draft_plan_json);
    assert_eq!(after.selected_draft_node_id, before.selected_draft_node_id);
}

#[tokio::test]
async fn failed_proposal_conversation_rolls_back_plan_and_draft() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("proposal_transaction_failure").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    let old_draft = apply_actions_to_draft(None, None, &normalize_actions(vec![
        json!({"type":"create_product", "name":"Original draft"}),
    ])).unwrap();
    persist_draft_state(&state.db, &session.session_id, Some(&old_draft), None).await.unwrap();
    let old_plan = json!({"assistant_response":"Old", "needs_confirmation":true, "clarification_question":null, "actions":[]}).to_string();
    crate::persistence::planner_repo::update_pending_plan(&state.db, &session.session_id, Some(&old_plan)).await.unwrap();
    let before = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.unwrap();
    let new_draft = apply_actions_to_draft(None, None, &normalize_actions(vec![
        json!({"type":"create_product", "name":"New draft"}),
    ])).unwrap();
    let new_plan = crate::services::planner_types::PlannerPlan {
        assistant_response: "New proposal".into(), needs_confirmation: true,
        clarification_question: None, actions: vec![],
    };
    sqlx::query("CREATE TRIGGER fail_proposal_assistant BEFORE INSERT ON planner_conversation_entries WHEN NEW.role = 'assistant' BEGIN SELECT RAISE(ABORT, 'injected failure'); END")
        .execute(&state.db).await.unwrap();
    assert!(crate::services::planner_session::persist_proposal(
        &state.db, &session.session_id, &new_plan, Some(&new_draft), None, "user request", "New proposal",
    ).await.is_err());
    let after = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.unwrap();
    assert_eq!(before.pending_plan_json, after.pending_plan_json);
    assert_eq!(before.draft_plan_json, after.draft_plan_json);
    assert!(crate::persistence::planner_repo::list_conversation_entries(&state.db, &session.session_id).await.unwrap().is_empty());
    sqlx::query("DROP TRIGGER fail_proposal_assistant").execute(&state.db).await.unwrap();
    crate::services::planner_session::persist_proposal(
        &state.db, &session.session_id, &new_plan, Some(&new_draft), None, "user request", "New proposal",
    ).await.unwrap();
    let saved = crate::services::planner_session::load_session_from_db(&state.db, &session.session_id).await.unwrap();
    assert!(saved.draft_plan.unwrap().nodes.iter().any(|node| node.name == "New draft"));
    assert_eq!(saved.conversation.iter().map(|entry| entry.content.as_str()).collect::<Vec<_>>(), ["user request", "New proposal"]);
}

#[tokio::test]
async fn failed_voice_clear_and_selection_preserve_session_state() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("voice_exchange_atomicity").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    let draft = apply_actions_to_draft(None, None, &normalize_actions(vec![
        json!({"type":"create_product", "name":"Voice product"}),
        json!({"type":"create_product_area", "target":"Voice product", "product_area_name":"Inventory"}),
    ])).unwrap();
    persist_draft_state(&state.db, &session.session_id, Some(&draft), None).await.unwrap();
    let pending = json!({"assistant_response":"Review", "needs_confirmation":true, "clarification_question":null, "actions":[]}).to_string();
    crate::persistence::planner_repo::update_pending_plan(&state.db, &session.session_id, Some(&pending)).await.unwrap();
    let before = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_voice_assistant BEFORE INSERT ON planner_conversation_entries WHEN NEW.role = 'assistant' BEGIN SELECT RAISE(ABORT, 'injected failure'); END")
        .execute(&state.db).await.unwrap();
    for command in ["clear draft", "select product area inventory"] {
        let outcome = submit_planner_voice_turn(state.planner_service.clone(), &state, session.session_id.clone(), command.into(), None, None).await;
        assert!(matches!(outcome, Err(ref error) if error.to_string().contains("injected failure")), "unexpected result for {command}: {outcome:?}");
        let after = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.unwrap();
        assert_eq!(after.pending_plan_json, before.pending_plan_json);
        assert_eq!(after.draft_plan_json, before.draft_plan_json);
        assert_eq!(after.selected_draft_node_id, before.selected_draft_node_id);
        assert!(crate::persistence::planner_repo::list_conversation_entries(&state.db, &session.session_id).await.unwrap().is_empty());
    }
    sqlx::query("DROP TRIGGER fail_voice_assistant").execute(&state.db).await.unwrap();
    let response = submit_planner_voice_turn(state.planner_service.clone(), &state, session.session_id.clone(), "clear draft.".into(), None, None).await.unwrap();
    assert_eq!(response.status, "execution");
    let saved = crate::services::planner_session::load_session_from_db(&state.db, &session.session_id).await.unwrap();
    assert!(saved.draft_plan.is_none() && saved.pending_plan.is_none());
    assert_eq!(saved.conversation.len(), 2);
}

#[tokio::test]
async fn failed_text_clarification_and_report_leave_no_partial_exchange() {
    let _guard = acquire_planner_test_lock().await;
    let state = make_test_state("text_exchange_atomicity").await;
    let session = create_planner_session(state.planner_service.clone(), &state.db, None, None).await.unwrap();
    sqlx::query("UPDATE planner_sessions SET provider_id = NULL, model_name = NULL WHERE id = ?")
        .bind(&session.session_id).execute(&state.db).await.unwrap();
    let draft = apply_actions_to_draft(None, None, &normalize_actions(vec![
        json!({"type":"create_product", "name":"Text fixture"}),
    ])).unwrap();
    persist_draft_state(&state.db, &session.session_id, Some(&draft), None).await.unwrap();
    let pending = json!({"assistant_response":"Review", "needs_confirmation":true, "clarification_question":null, "actions":[]}).to_string();
    crate::persistence::planner_repo::update_pending_plan(&state.db, &session.session_id, Some(&pending)).await.unwrap();
    let before = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_text_assistant BEFORE INSERT ON planner_conversation_entries WHEN NEW.role = 'assistant' BEGIN SELECT RAISE(ABORT, 'injected failure'); END")
        .execute(&state.db).await.unwrap();
    for input in ["Explain this", "show status"] {
        let outcome = crate::services::planner_service::submit_planner_turn(state.planner_service.clone(), &state, session.session_id.clone(), input.into(), None, None).await;
        assert!(matches!(outcome, Err(ref error) if error.to_string().contains("injected failure")), "unexpected result for {input}: {outcome:?}");
        let after = crate::persistence::planner_repo::get_session(&state.db, &session.session_id).await.unwrap();
        assert_eq!(after.pending_plan_json, before.pending_plan_json);
        assert_eq!(after.draft_plan_json, before.draft_plan_json);
        assert!(crate::persistence::planner_repo::list_conversation_entries(&state.db, &session.session_id).await.unwrap().is_empty());
    }
}
