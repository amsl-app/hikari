use url::Url;

pub(crate) mod bots;
pub(crate) mod csml;
pub(crate) mod modules;
pub(crate) mod opt;

#[derive(Clone, Debug)]
pub(crate) struct WorkerUrl(pub Url);
