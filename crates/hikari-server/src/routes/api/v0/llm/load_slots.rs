use hashbrown::HashMap;
use hashbrown::hash_map::EntryRef;
use hikari_config::module::session::Session;
use hikari_config::module::{Module, ModuleConfig};
use hikari_llm::builder::slot::{LoadToSlot, ValueSource};
use hikari_llm::execution::error::LlmExecutionError;
use hikari_model::user::User;
use hikari_utils::values::{QueryYaml, ValueDecoder};
use sea_orm::DatabaseConnection;
use serde::Serialize;
use uuid::Uuid;
use yaml_serde::Value;

macro_rules! gen_match_user_fields {
    ($x:expr, $t:ident, $( $idents:ident ),*) => {
        match $x {
            $(
                stringify!($idents) => yaml_serde::to_value(&$t.$idents)?,
            )*
            _ => Err(LlmExecutionError::Unexpected($x.to_string()))?,
        }
    };
}

async fn get_user_config(conn: &DatabaseConnection, user: &User) -> Result<Value, LlmExecutionError> {
    let config = hikari_db::config::Query::get_user_config(conn, user.id).await?;
    let config_map = config
        .into_iter()
        .map(|c| {
            tracing::trace!(?c.key, "Decoding user config value for load_slots");
            (Value::String(c.key), Value::decode(&c.value))
        })
        .collect();
    Ok(Value::Mapping(config_map))
}

pub async fn load_slots<'a>(
    conn: &DatabaseConnection,
    conversation_id: Uuid,
    slots: &Vec<LoadToSlot>,
    module_config: &ModuleConfig,
    user: &User,
    module: &'a Module<'a>,
    session: &'a Session,
) -> Result<(), LlmExecutionError> {
    let current_module_id = &module.id;
    let current_session_id = &session.id;

    let mut module_map = HashMap::from([(current_module_id.to_owned(), yaml_serde::to_value(module)?)]);
    let mut session_map = HashMap::from([(
        session_key(current_module_id, current_session_id),
        yaml_serde::to_value(session)?,
    )]);

    let user_config = get_user_config(conn, user).await?;

    for LoadToSlot { name, source } in slots {
        let value = match &source {
            ValueSource::Session(session_path) => {
                let module_id = session_path.module.get_id(&module.id)?;
                let session_id = session_path.session.get_id(&session.id)?;
                let key = session_key(&module_id, &session_id);
                let session = module_config
                    .modules()
                    .get(&module_id)
                    .and_then(|m| m.sessions.get(&session_id))
                    .ok_or_else(|| LlmExecutionError::SessionNotFound(key.clone()))?;
                let session: &Value = get_or_insert(&mut session_map, &key, session)?;
                session.query(&session_path.path)?
            }
            ValueSource::Module(module_path) => {
                let module_id = module_path.module.get_id(&module.id)?;
                let module = module_config
                    .modules()
                    .get(&module_id)
                    .ok_or_else(|| LlmExecutionError::ModuleNotFound(module_id.clone()))?;
                let module = get_or_insert(&mut module_map, &module_id, module)?;
                module.query(&module_path.path)?
            }
            ValueSource::User(user_path) => gen_match_user_fields!(
                user_path.path.as_str(),
                user,
                id,
                onboarding,
                name,
                birthday,
                current_module,
                groups
            ),
            ValueSource::UserConfig(user_conf_path) => {
                let user_conf_path = user_conf_path.path.as_str();
                user_config.query(user_conf_path)?
            }
        };

        let string_value = value.encode();

        tracing::debug!(?name, "Setting slot from load_slots");

        hikari_db::llm::slot::conversation_slot::Mutation::insert_or_update_slot(
            conn,
            conversation_id,
            name.clone(),
            string_value,
        )
        .await?;
    }
    Ok(())
}

fn session_key(module_id: &str, session_id: &str) -> String {
    format!("{module_id}_{session_id}")
}

fn get_or_insert<'a, T>(
    map: &'a mut hashbrown::HashMap<String, Value>,
    key: &str,
    value: T,
) -> Result<&'a Value, yaml_serde::Error>
where
    T: Serialize,
{
    let value = match map.entry_ref(key) {
        EntryRef::Occupied(entry) => entry.into_mut(),
        EntryRef::Vacant(entry) => entry.insert(yaml_serde::to_value(value)?),
    };
    Ok(value)
}
