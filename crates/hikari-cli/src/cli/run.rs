use crate::cli::run::prompt::SimplePrompt;
use crate::opt::Run;
use anyhow::{Error, Result, anyhow};
use core::option::Option;
use core::option::Option::{None, Some};
use core::result::Result::{Err, Ok};
use csml_engine::data::models::{BotOpt, CsmlRequest};
use csml_engine::future::db_connectors::clean_db::delete_expired_data;
use csml_engine::future::db_connectors::conversations::close_all_conversations;
use csml_engine::future::db_connectors::{conversations, init_db};
use csml_engine::future::start_conversation_stream;
use csml_engine::make_migrations;
use csml_interpreter::data::Client;
use csml_interpreter::data::csml_bot::CsmlBot;
use csml_interpreter::data::csml_result::CsmlResult;
use csml_interpreter::validate_bot;
use futures::{StreamExt, pin_mut};
use hikari_common::csml_utils;
use hikari_common::csml_utils::load_flows_from_dir;
use hikari_common::error::BotError;
use reedline::{Reedline, Signal};
use serde_json::json;
use serde_json::value::Value;
use std::path::PathBuf;

pub(crate) mod prompt;

enum Command {
    Exit,
    Flow(String),
    Message(String),
}

fn init_request(client: Client, string: Option<&str>) -> CsmlRequest {
    let payload = match string {
        None => {
            json!({"content_type": "flow_trigger", "content": {"flow_id": "*"}})
        }
        Some(_) => {
            json!({
                "content_type": "text",
                "content": { "text": string},
            })
        }
    };
    csml_utils::init_request(client, payload, json!({}))
}

async fn load_bot(name: String, endpoint: Option<String>) -> Result<CsmlBot, BotError> {
    let flows = load_flows_from_dir(&PathBuf::from("CSML").join(&name)).await?;

    csml_utils::init_bot(name, flows, endpoint)
}

pub(crate) async fn run(opt: Run) -> Result<(), Error> {
    let _guard = opt.debug.then(|| {
        hikari_utils::tracing::setup(
            hikari_utils::tracing::TracingConfig::builder()
                .package(env!("CARGO_PKG_NAME"))
                .version(env!("CARGO_PKG_VERSION"))
                .build(),
        )
    });

    let mut line: String = String::new();

    let mut bot = load_bot(opt.bot.clone(), opt.endpoint).await?;
    let CsmlResult { errors, .. } = validate_bot(&bot);

    if let Some(flow) = opt.flow {
        bot.default_flow = flow;
    }

    println!(
        "Loaded bot. Flows:\n{}",
        bot.flows
            .iter()
            .map(|flow| flow.name.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );

    if !errors.is_empty() {
        for error in errors {
            eprintln!("{error:?}");
        }
        return Err(anyhow!("Bot contains errors"));
    }

    make_migrations().map_err(|e| anyhow!("Engine Error: {e:?}"))?;

    let client = Client {
        user_id: "other".to_owned(),
        bot_id: opt.bot,
        channel_id: "some-channel-id".to_owned(),
    };

    println!("Starting bot. User Id: {}, Bot Id: {}", client.user_id, client.bot_id);

    let mut db = init_db().await?;

    if let Some(conv) = conversations::get_latest_open(&client, &mut db)
        .await
        .map_err(|e| anyhow!("Engine Error: {e:?}"))?
    {
        println!("Found existing conversation {}", conv.id);
    } else {
        println!("No existing conversation");
    }

    let mut line_editor = Reedline::create();
    let prompt = SimplePrompt;

    let command_regex = regex::Regex::new(r"^/(\w+)(?:\s+(.*))?$")?;

    loop {
        let run_opt = BotOpt::CsmlBot(Box::new(bot.clone()));

        let sig = line_editor.read_line(&prompt)?;
        match sig {
            Signal::Success(user_input) => {
                if opt.debug {
                    println!("Got input {user_input}");
                }

                let command = match command_regex.captures(&user_input) {
                    Some(captures) => {
                        let command = captures.get(1).expect("missing first arg").as_str();
                        let arg = captures.get(2).map(|m| m.as_str());
                        match command {
                            "exit" => Command::Exit,
                            "flow" => Command::Flow(arg.expect("missing second arg").to_owned()),
                            "message" => Command::Message(arg.expect("missing second arg").to_owned()),
                            _ => {
                                eprintln!("Unknown command {command}");
                                continue;
                            }
                        }
                    }
                    None => Command::Message(user_input),
                };

                let user_input = match command {
                    Command::Exit => {
                        print!("Exiting...");
                        break;
                    }
                    Command::Flow(flow) => {
                        print!("Setting flow to {flow}");
                        bot.default_flow = flow;
                        close_all_conversations(&client, &mut db).await?;
                        continue;
                    }
                    Command::Message(message) => message,
                };

                match start_conversation_stream(init_request(client.clone(), Some(&user_input)), run_opt, &mut db).await
                {
                    Ok((conversation, stream_data)) => {
                        println!("Conversation state: {conversation:?}");
                        let Some(mut stream_data) = stream_data else {
                            eprintln!("Conversation delayed");
                            break;
                        };
                        {
                            let res = stream_data.stream().await;
                            let stream = match res {
                                Ok(stream) => stream,
                                Err(err) => {
                                    eprintln!("Failed to initialize stream:\n{err:?}");
                                    break;
                                }
                            };
                            pin_mut!(stream);
                            while let Some(message) = stream.next().await {
                                let message = message?;
                                println!(
                                    "({} {}",
                                    message.payload.content_type,
                                    message.payload.content.unwrap_or(Value::Null)
                                );
                            }
                        }
                        let conversation = stream_data.finalize().await?;
                        println!("Conversation updated:\n{conversation:?}");
                        if conversation.is_closed() {
                            println!("Conversation closed");
                        }
                        // for (key, value) in obj.iter() {
                        //     println!("{}: {}", key, value)
                        // }
                    }
                    Err(err) => {
                        eprintln!("{err:?}");
                        break;
                    }
                }
                line.clear();
            }
            Signal::CtrlD | Signal::CtrlC => {
                println!("\nAborted!");
                break;
            }
        }
    }

    delete_expired_data(&mut db).await.ok();

    Ok(())
}
