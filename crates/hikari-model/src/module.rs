use crate::module::assessment::instance::ModuleAssessmentInstance;
use crate::module::session::SessionFull;
use crate::module::{assessment::ModuleAssessmentFull, session::instance::SessionInstance};
use chrono::{DateTime, TimeZone, Utc};
use hikari_config::module::Module;
use hikari_config::module::assessment::ModuleAssessment;
use hikari_config::module::unlock::{
    LockedUntil, UnlockTrigger, UnlockTriggerMode, UnlockTriggerTimeFormat, UnlockTriggerWait,
};
use hikari_config::{
    generic::{Metadata, Theme},
    module::{ModuleCategory, unlock::Unlock},
};

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;

pub mod assessment;
pub mod group;
pub mod instance;
pub mod session;

#[derive(Serialize, ToSchema)]
pub struct ModuleFull<'a> {
    #[schema(example = "module-id")]
    pub id: &'a str,

    #[schema(example = "string")]
    pub title: &'a str,

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

    #[schema(example = "session-id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "default-session")]
    pub default_session: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions: Option<Vec<SessionFull<'a>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<&'a Theme>,

    pub hidden: bool,
    #[serde(default = "ModuleCategory::default_type")]
    pub category: ModuleCategory,

    pub weight: usize,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessment: Option<ModuleAssessmentFull<'a>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<&'a Metadata>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "HashSet::is_empty")]
    pub groups: HashSet<&'a str>,

    #[serde(rename = "self-learning")]
    pub self_learning: bool,

    pub quizzable: bool,
}

impl<'a> ModuleFull<'a> {
    pub fn from_config<'mg: 'a>(
        module: &'a Module,
        deep: bool,
        module_entries: &'a [SessionInstance],
        assessment: Option<&'a ModuleAssessmentInstance>,
        completion: Option<DateTime<Utc>>,
    ) -> Self {
        let sessions = deep.then(|| {
            module
                .sessions
                .values()
                .map(|s| SessionFull::from_config(s, &module.id, module_entries))
                .collect()
        });

        ModuleFull {
            id: &module.id,
            title: &module.title,
            subtitle: module.subtitle.as_deref(),
            description: module.description.as_deref(),
            icon: module.icon.as_deref(),
            banner: module.banner.as_deref(),
            default_session: module.default_session.as_deref(),
            hidden: module.hidden,
            category: module.category,
            assessment: module
                .assessment
                .as_ref()
                .map(|assessment_config| new_module_assessment(&assessment_config.borrowed(), assessment)),
            weight: module.weight.unwrap_or(1),
            sessions,
            completion,
            theme: module.theme.as_ref(),
            metadata: module.metadata.as_ref(),
            groups: module.module_groups.iter().map(String::as_str).collect(),
            self_learning: module.self_learning,
            quizzable: module.quizzable,
        }
    }
}

pub(crate) fn new_module_assessment<'a>(
    config: &ModuleAssessment<'a>,
    assessment: Option<&ModuleAssessmentInstance>,
) -> ModuleAssessmentFull<'a> {
    let (last_pre, last_post) = match assessment {
        None => (None, None),
        Some(assessment) => (assessment.last_pre, assessment.last_post),
    };
    ModuleAssessmentFull {
        pre: config.pre.clone(),
        post: config.post.clone(),
        last_pre,
        last_post,
    }
}

#[allow(clippy::unwrap_in_result)]
pub fn locked_until(unlocked: &Unlock, module_session_entries: &[SessionInstance]) -> Option<LockedUntil> {
    let entries: HashMap<&str, &SessionInstance> =
        module_session_entries.iter().map(|e| (e.session.as_str(), e)).collect();

    let locks = unlocked.triggers.iter().map(|trigger| match trigger {
        UnlockTrigger::Time { after } => match after {
            UnlockTriggerTimeFormat::Local(date) => LockedUntil::Time(Utc.from_utc_datetime(date)),
            UnlockTriggerTimeFormat::Utc(date) => LockedUntil::Time(*date),
        },
        UnlockTrigger::Completion { after, wait } => {
            // We only support one session for now and made sure that it has length 1 during validation
            let after = after.first().cloned().unwrap_or(String::new());
            if let Some(instance) = entries.get(after.as_str()).copied() {
                if let Some(completion_time) = instance.completion {
                    let completion_time = match wait {
                        Some(UnlockTriggerWait::Days(days)) => {
                            // We add the days and set hours, minutes, and seconds to zero
                            let naive = completion_time.date_naive().and_hms_opt(0, 0, 0).expect("Invalid date")
                                + chrono::Duration::days(i64::from(*days));
                            Utc.from_utc_datetime(&naive)
                        }
                        Some(UnlockTriggerWait::Seconds(seconds)) => {
                            completion_time + chrono::Duration::seconds(i64::from(*seconds))
                        }
                        None => completion_time,
                    };
                    LockedUntil::Time(completion_time)
                } else {
                    LockedUntil::Undefined
                }
            } else {
                LockedUntil::Undefined // If the session status is not found, the session it not started yet
            }
        }
    });

    let future_locks: Vec<_> = locks
        .filter(|lock| match lock {
            LockedUntil::Time(time) => *time > Utc::now(),
            LockedUntil::Undefined => true,
        })
        .collect();

    if future_locks.is_empty() {
        None
    } else {
        match unlocked.trigger_mode {
            UnlockTriggerMode::Any => {
                // Earliest unlock time among future ones (or Undefined)
                future_locks
                    .iter()
                    .filter_map(|lock| match lock {
                        LockedUntil::Time(time) => Some(*time),
                        LockedUntil::Undefined => None,
                    })
                    .min()
                    .map(LockedUntil::Time)
                    .or(Some(LockedUntil::Undefined))
            }
            UnlockTriggerMode::All => {
                // Latest unlock time, unless any are undefined
                if future_locks.iter().any(|l| matches!(l, LockedUntil::Undefined)) {
                    Some(LockedUntil::Undefined)
                } else {
                    future_locks
                        .iter()
                        .filter_map(|lock| match lock {
                            LockedUntil::Time(time) => Some(*time),
                            #[allow(unreachable_code)]
                            LockedUntil::Undefined => None,
                        })
                        .max()
                        .map(LockedUntil::Time)
                }
            }
        }
    }
}
