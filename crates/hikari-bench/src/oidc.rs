use hikari::OidcClientType;
use openidconnect::core::{CoreClient, CoreProviderMetadata};
use openidconnect::{ClientId, ClientSecret, IssuerUrl};

pub async fn create_client(
    oidc_issuer_url: String,
    oidc_client_id: String,
    oidc_client_secret: Option<String>,
) -> anyhow::Result<OidcClientType> {
    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let provider_metadata =
        CoreProviderMetadata::discover_async(IssuerUrl::new(oidc_issuer_url)?, &http_client).await?;
    let oidc_client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(oidc_client_id),
        oidc_client_secret.map(ClientSecret::new),
    );
    Ok(oidc_client)
}
