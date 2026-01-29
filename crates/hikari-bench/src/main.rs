mod error;
mod oidc;
mod opt;
mod user;

use crate::opt::{Cli, Commands, Stress};
use crate::user::User;
use anyhow::{Result, anyhow};
use clap::Parser;
use csv_async::AsyncReaderBuilder;
use hikari::{Config, OidcClient};
use hikari::{PublicClient, SecureClient};

use crate::error::Error;
use futures::future::try_join_all;
use std::path::Path;
use tokio::try_join;
use tokio_stream::StreamExt;
use tracing::Level;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use url::Url;

async fn read_csv<P: AsRef<Path>>(path: P) -> Result<Vec<User>> {
    let rdr = AsyncReaderBuilder::new()
        .delimiter(b';')
        .create_deserializer(tokio::fs::File::open(path).await?);
    let mut result = Vec::new();
    let mut records = rdr.into_deserialize();
    while let Some(record) = records.next().await {
        result.push(record?);
    }
    Ok(result)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let layer = tracing_subscriber::fmt::layer();
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(LevelFilter::TRACE)
        .with_target("hyper", Level::INFO);

    tracing_subscriber::registry()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with(layer.with_filter(LevelFilter::DEBUG))
        .with(filter)
        // completes the builder.
        .init();

    match cli.command {
        Commands::Stress(Stress {
            users,
            oidc_issuer_url,
            oidc_client_id,
            oidc_client_secret,
            count,
            module,
            session,
        }) => {
            let (users, oidc_client) = try_join!(
                read_csv(users),
                oidc::create_client(oidc_issuer_url, oidc_client_id, oidc_client_secret)
            )?;

            let len = users.len();
            if let Some(count) = count
                && count > len
            {
                return Err(anyhow!("Not enough users in source csv."));
            }

            let clients: Result<Vec<_>, url::ParseError> = users
                .into_iter()
                .take(count.unwrap_or(len))
                .map(|user| {
                    let client = OidcClient::new(oidc_client.clone(), user.into(), Config::new(Url::parse(&cli.url)?));
                    Ok(client)
                })
                .collect();
            let clients: Result<_, Error> = try_join_all(clients?.into_iter().map(|client| async move {
                let token = client.fetch_token().await?;
                Ok((client, token))
            }))
            .await;
            let (clients, tokens): (Vec<_>, Vec<_>) = clients?.into_iter().unzip();
            println!("{tokens:?}");

            let module = module.as_str();
            let session = session.as_str();
            let messages = try_join_all(
                clients
                    .into_iter()
                    .map(|client| async move { client.start_session(module, session).await.map_err(Error::from) }),
            )
            .await?;

            println!("{messages:?}");
        }
        Commands::Status => {
            let client = hikari::SimpleClient::new(Config::new(Url::parse(&cli.url)?));
            let status = client.get_status().await?;
            println!("{status:?}");
        }
    }

    Ok(())
}
