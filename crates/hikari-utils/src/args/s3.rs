use clap::Args;
use url::Url;

#[derive(Debug, Clone, Args)]
#[allow(clippy::struct_field_names)]
pub struct S3 {
    #[arg(long, required = false)]
    pub s3_endpoint: Url,
    #[arg(long, required = false)]
    pub s3_region: String,
    #[arg(long, required = false)]
    pub s3_access_key: String,
    #[arg(long, required = false)]
    pub s3_secret_key: String,
}
