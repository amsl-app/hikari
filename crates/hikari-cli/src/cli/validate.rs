use crate::opt::Validate;
use anyhow::{Error, Result, anyhow};
use csml_interpreter::data::{CsmlBot, CsmlFlow, CsmlResult};
use csml_interpreter::validate_bot;
use hikari_common::csml_utils;
use std::path::Path;

async fn bot_from_paths<P: AsRef<Path>>(flows: &[P], prefix: Option<String>) -> Result<CsmlBot, Error> {
    let bot = CsmlBot {
        id: "a".to_owned(),
        name: "a".to_owned(),
        apps_endpoint: None,
        flows: futures::future::try_join_all(flows.iter().map(|path| csml_utils::load_flow(path.as_ref())).map(
            |flow| async {
                let flow = flow.await;
                if let Some(prefix) = &prefix {
                    return flow.map(|flow| CsmlFlow {
                        name: format!("{prefix}{}", &flow.name),
                        ..flow
                    });
                }
                flow
            },
        ))
        .await?,
        native_components: None,
        custom_components: None,
        default_flow: "a".to_owned(),
        bot_ast: None,
        no_interruption_delay: None,
        env: None,
        modules: None,
        multibot: None,
    };

    Ok(bot)
}

pub(crate) async fn validate(opt: Validate) -> Result<(), Error> {
    let bot = bot_from_paths(&opt.paths, opt.prefix).await?;
    let flows: Vec<_> = bot.flows.iter().map(|flow| &flow.name).collect();
    let CsmlResult { warnings, errors, .. } = validate_bot(&bot);

    for warning in &warnings {
        eprintln!("{warning:?}");
    }

    if !errors.is_empty() {
        for error in &errors {
            eprintln!("{error:?}");
        }
        return Err(anyhow!("Bot containing the flows {flows:?} has errors"));
    }
    if opt.strict && !warnings.is_empty() {
        return Err(anyhow!("Bot containing the flows {flows:?} has warnings"));
    }
    println!("Bot containing the flows {flows:?} is ok 👌");
    Ok(())
}
