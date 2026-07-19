use super::api;
use super::global;
use super::login;

use anyhow::Result;
use axum::Router;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder};
use utoipa::{Modify, OpenApi, openapi::security::SecurityScheme};
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

struct SecurityAddon;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::v0::status::get_status,
        api::v0::assessment::list_assessments,
        api::v0::assessment::list_user_assessments,
        api::v0::assessment::start,
        api::v0::assessment::get_scales,
        api::v0::assessment::load,
        api::v0::assessment::update,
        api::v0::assessment::submit,
        api::v0::quiz::get_quizzes,
        api::v0::quiz::get_scores,
        api::v0::quiz::get_quiz,
        api::v0::quiz::get_questions,
        api::v0::quiz::get_question,
        api::v0::quiz::get_next_question,
        api::v0::quiz::submit_answer,
        api::v0::quiz::add_feedback,
        api::v0::quiz::skip_question,
        api::v0::bots::message::message,
        api::v0::bots::list_bots,
        api::v0::bots::flows::list_flows,
        api::v0::bots::conversations::get_conversations,
        api::v0::bots::conversations::get_open_conversations,
        api::v0::bots::flows::trigger_flow,
        api::v0::modules::list_modules,
        api::v0::modules::list_groups,
        api::v0::modules::get_module,
        api::v0::modules::get_session_data,
        api::v0::modules::history,
        api::v0::modules::abort_all_sessions,
        api::v0::modules::finish_session,
        api::v0::modules::next_session_custom,
        api::v0::modules::flow_custom,
        api::v0::modules::list_finished_modules,
        api::v0::modules::get_source,
        api::v0::modules::assessment::pre_post_assessment,
        api::v0::modules::assessment::start_module_assessment,
        api::v0::modules::assessment::submit_module_assessment,
        api::v0::modules::quiz::start_quiz,
        api::v0::modules::quiz::get_module_scores,
        api::v0::modules::quiz::get_module_quizzes,
        api::v0::user::get_user_info,
        api::v0::user::update_user_info,
        api::v0::user::access::add_access,
        api::v0::user::access::access_approvals,
        api::v0::user::config::get_user_configs,
        api::v0::user::config::get_user_config_value,
        api::v0::user::config::set_user_config,
        api::v0::user::config::delete_user_config,
        api::v0::user::handle::get_user_handle,
        api::v0::user::context_log::get_user_context_logs,
        api::v0::user::context_log::get_latest_user_context_log,
        api::v0::user::context_log::get_user_context_logs_by_type,
        api::v0::user::context_log::add_user_context_log,
        api::v0::user::context_log::get_latest_user_context_log_by_type,
        api::v0::modules::messaging::start_session,
        api::v0::modules::messaging::reset_session,
        api::v0::modules::messaging::chat_session,
        api::v0::modules::messaging::chat_session_v2,
        api::v0::modules::messaging::chat_session_ws,
        api::v0::modules::messaging::abort_session,
        api::v0::journal::get_journal_entries,
        api::v0::journal::create_journal_entry,
        api::v0::journal::create_empty_journal_entry,
        api::v0::planner::get_planner_entries,
        api::v0::planner::get_planner_entry,
        api::v0::planner::create_planner_entry,
        api::v0::planner::create_planner_entries_bulk,
        api::v0::planner::update_planner_entry,
        api::v0::planner::delete_planner_entry,
        api::v0::planner::get_ical_token,
        api::v0::planner::delete_ical_token,
        api::v0::planner::get_planner_ical,
        api::v0::planner::planner_assistant,
        api::v0::journal::journal_entry::get_journal_entry,
        api::v0::journal::journal_entry::list_journal_entry_contents,
        api::v0::journal::journal_entry::add_journal_entry_content,
        api::v0::journal::journal_entry::get_journal_entry_content,
        api::v0::journal::journal_entry::focus::get_focus,
        api::v0::journal::journal_entry::focus::set_focus,
        api::v0::journal::journal_entry::mood::get_mood,
        api::v0::journal::journal_entry::mood::set_mood,
        api::v0::journal::journal_entry::mood::unset_mood,
        api::v0::journal::journal_focus::get_user_focus,
        api::v0::journal::journal_focus::create_user_focus,
        api::v0::journal::journal_focus::update_user_focus,
        api::v0::journal::journal_focus::get_focus_incl_global,
        api::v0::journal::assistant::prompt,
        api::v0::journal::assistant::merge,
        api::v0::journal::assistant::text_prompt,
        api::v0::journal::assistant::text_merge,
        api::v0::journal::assistant::summarize::summarize_handler,
        login::login_token,
        login::logout,
        global::frontend_version,
    ),
    modifiers(&SecurityAddon),
    tags()
)]
struct ApiDoc;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // we can unwrap safely, since there already are components registered.
        let components = openapi.components.as_mut().expect("components not registered");
        components.add_security_scheme(
            "token",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .description(Some("Api Token"))
                    .build(),
            ),
        );
    }
}

pub fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
        // There is no need to create `RapiDoc::with_openapi` because the OpenApi is served
        // via SwaggerUi instead we only make rapidoc to point to the existing doc.
        .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
}

pub(crate) fn openapi_json(pretty: bool) -> Result<String> {
    let openapi = ApiDoc::openapi();
    let json = if pretty {
        serde_json::to_string_pretty(&openapi)?
    } else {
        serde_json::to_string(&openapi)?
    };
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::openapi_json;
    use std::collections::HashSet;

    #[test]
    fn openapi_json_is_valid_json() {
        let json = openapi_json(false).expect("openapi generation should succeed");
        let value: serde_json::Value = serde_json::from_str(&json).expect("openapi output should be valid json");

        assert!(value.get("openapi").and_then(serde_json::Value::as_str).is_some());
        assert!(value.get("paths").and_then(serde_json::Value::as_object).is_some());
    }

    #[test]
    fn openapi_operation_ids_are_unique() {
        let json = openapi_json(false).expect("openapi generation should succeed");
        let value: serde_json::Value = serde_json::from_str(&json).expect("openapi output should be valid json");

        let paths = value
            .get("paths")
            .and_then(serde_json::Value::as_object)
            .expect("openapi paths should be an object");

        let mut seen = HashSet::<String>::new();
        for path_item in paths.values() {
            let path_obj = path_item.as_object().expect("openapi path item should be an object");
            let path_value = serde_json::Value::Object(path_obj.clone());

            for method in ["get", "post", "put", "patch", "delete"] {
                let operation_value = &path_value[method];
                if operation_value.is_null() {
                    continue;
                }

                let operation_id = operation_value["operationId"]
                    .as_str()
                    .expect("openapi operation should have an operationId");

                assert!(
                    seen.insert(operation_id.to_string()),
                    "duplicate operationId found: {operation_id}"
                );
            }
        }
    }
}
