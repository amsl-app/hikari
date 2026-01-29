use std::collections::HashSet;

use crate::module::{
    locked_until,
    session::instance::{SessionInstance, SessionInstanceStatus},
};
use chrono::{DateTime, Utc};
use hikari_config::{
    generic::{Metadata, Theme},
    module::{
        content::{Content, ContentSource},
        llm_agent::LlmService,
        session::Session,
        unlock::{LockedUntil, Unlock},
    },
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod instance;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Copy, ToSchema)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
// Older frontend version can only handle OpenAI and Local. Currently Local Providers dont have to be handled separately.
pub enum PublicLlmProvider {
    #[default]
    OpenAI,
    Local,
}

impl From<LlmService> for PublicLlmProvider {
    fn from(service: LlmService) -> Self {
        match service {
            LlmService::OpenAI => PublicLlmProvider::OpenAI,
            _ => PublicLlmProvider::Local, // Treat all other services as Local for public API
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct SessionFull<'a> {
    #[schema(example = "session-id")]
    pub id: &'a str,

    #[schema(example = "title")]
    pub title: &'a str,

    #[schema(example = "bot/flow")]
    pub bot: Option<&'a str>,

    #[schema(example = "subtitle")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<&'a str>,

    #[schema(example = "description")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'a str>,

    #[schema(example = "icon-url")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<&'a str>,

    #[schema(example = "banner-url")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<&'a Theme>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<&'a i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<DateTime<Utc>>,

    #[serde(rename = "next-session", skip_serializing_if = "Option::is_none")]
    pub next_session: Option<&'a str>,

    pub status: SessionInstanceStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlock: Option<&'a Unlock>,

    #[serde(rename = "locked-until", skip_serializing_if = "Option::is_none")]
    pub locked_until: Option<LockedUntil>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<&'a Metadata>,

    pub llm: bool,

    pub hidden: bool,

    pub quizzable: bool,

    #[serde(skip_serializing_if = "Option::is_none", rename = "llm-provider")]
    pub llm_provider: Option<PublicLlmProvider>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<&'a ContentSource>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<&'a str>,
}

impl<'a> SessionFull<'a> {
    #[must_use]
    pub fn from_config(session: &'a Session, module_id: &'a str, entries: &'a [SessionInstance]) -> Self {
        let module_session_entries = entries
            .iter()
            .filter(|e| e.module == module_id)
            .cloned()
            .collect::<Vec<SessionInstance>>();

        let current_entry = module_session_entries.iter().find(|e| e.session == session.id);

        let (status, completion) =
            current_entry.map_or((SessionInstanceStatus::NotStarted, None), |m| (m.status, m.completion));

        // Important: The user has to reload the module to see newly unlocked contents
        let unlocked_contents = session
            .contents
            .iter()
            .filter(|c| {
                c.unlock
                    .as_ref()
                    .is_none_or(|unlock| locked_until(unlock, &module_session_entries).is_none())
            })
            .collect::<Vec<&Content>>();

        let sources: HashSet<&ContentSource> = unlocked_contents.iter().flat_map(|c| &c.sources.primary).collect();
        let topics: Vec<&str> = unlocked_contents.iter().map(|c| c.title.as_ref()).collect();

        let locked_until = session
            .unlock
            .as_ref()
            .and_then(|unlock| locked_until(unlock, &module_session_entries));

        SessionFull {
            id: &session.id,
            title: &session.title,
            bot: session.bot.as_deref(),
            subtitle: session.subtitle.as_deref(),
            description: session.description.as_deref(),
            icon: session.icon.as_deref(),
            banner: session.banner.as_deref(),
            theme: session.theme.as_ref(),
            time: session.time.as_ref(),
            next_session: session.next_session.as_deref(),
            unlock: session.unlock.as_ref(),
            locked_until,
            metadata: session.metadata.as_ref(),
            llm: session.llm_agent.is_some(),
            llm_provider: session.llm_agent.as_ref().map(|agent| agent.provider.clone().into()),
            topics: topics.into_iter().collect(),
            sources: sources.into_iter().collect(), // Only primary sources for the api
            status,
            completion,
            hidden: session.hidden,
            quizzable: session.quizzable,
        }
    }
}
