use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::{
    generic::{Metadata, Theme},
    module::{content::Content, error::ModuleError, llm_agent::LlmAgent, unlock::Unlock, v01::session::SessionV01},
};

#[derive(Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub bot: Option<String>,
    #[allow(clippy::struct_field_names)]
    pub next_session: Option<String>,
    pub theme: Option<Theme>,
    pub time: Option<i32>,
    pub unlock: Option<Unlock>,
    pub metadata: Option<Metadata>,
    pub hidden: bool,
    pub quizzable: bool,
    pub contents: Vec<Content>,
    #[serde(flatten)]
    pub llm_agent: Option<LlmAgent>,
    pub custom: Option<HashMap<String, serde_yml::Value>>,
}

impl Session {
    pub(crate) fn from_v01(session: SessionV01, all_contents: &[Content]) -> Result<Self, ModuleError> {
        let contents = session
            .contents
            .into_iter()
            .map(|content| {
                all_contents
                    .iter()
                    .find(|c| c.id == content)
                    .cloned()
                    .ok_or(ModuleError::ContentNotFound)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            id: session.id,
            title: session.title,
            subtitle: session.subtitle,
            description: session.description,
            icon: session.icon,
            banner: session.banner,
            bot: session.bot,
            next_session: session.next_session,
            theme: session.theme,
            time: session.time,
            unlock: session.unlock,
            metadata: session.metadata,
            contents,
            llm_agent: session.llm_agent,
            custom: session.custom,
            hidden: session.hidden,
            quizzable: session.quizzable,
        })
    }

    #[must_use]
    pub fn get_id(&self) -> &str {
        self.id.as_str()
    }

    #[must_use]
    pub fn next_session(&self) -> Option<&str> {
        self.next_session.as_deref()
    }

    #[must_use]
    pub fn bot_flow(&self) -> Option<&str> {
        self.bot.as_deref()
    }

    fn get_bot_parts(&self) -> Option<std::str::Split<'_, char>> {
        self.bot.as_ref().map(|bot| bot.split('/'))
    }

    fn get_bot_part(&self, n: usize) -> Option<&str> {
        self.get_bot_parts().and_then(|mut parts| parts.nth(n))
    }

    #[must_use]
    pub fn get_bot(&self) -> Option<&str> {
        self.get_bot_part(0)
    }

    #[must_use]
    pub fn get_bot_and_flow(&self) -> Option<(&str, Option<&str>)> {
        let mut parts = self.get_bot_parts()?;
        let bot = parts.next();
        bot.map(|bot| (bot, parts.next()))
    }

    pub fn validate(
        &self,
        bots: &HashMap<&String, Vec<&String>>,
        llm_agents: &HashSet<&String>,
    ) -> Result<(), ModuleError> {
        let Some((bot_id, flow_id)) = self.get_bot_and_flow() else {
            if let Some(llm_agent) = &self.llm_agent
                && !llm_agents.contains(&llm_agent.llm_agent)
            {
                tracing::error!(session_id = self.id, "session has no bot and is not using llm");
                return Err(ModuleError::LlmAgentNotFound);
            }
            return Ok(());
        };

        let bot = bots.get(&bot_id.to_owned()).ok_or(ModuleError::BotNotFound)?;
        tracing::debug!(bot_id = %bot_id, session_id = %self.id, "matching bot to session");
        let flow_id = flow_id.ok_or(ModuleError::FlowNotFound)?;
        bot.iter()
            .find(|f| f.as_str().eq(flow_id))
            .ok_or(ModuleError::FlowNotFound)?;

        Ok(())
    }
}
