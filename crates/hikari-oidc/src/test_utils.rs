use serde::de::DeserializeOwned;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::LazyLock;

static ROOT_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/test");
    path
});

pub(crate) fn load_json<T>(name: &str) -> T
where
    T: DeserializeOwned,
{
    let path = ROOT_PATH.join(name);
    tracing::info!(?path, "loading test json file");
    let reader = BufReader::new(
        File::open(&path)
            .inspect_err(|error| tracing::error!(error = error as &dyn Error, ?path, "failed to open json file"))
            .unwrap(),
    );
    let value = serde_json::from_reader(reader).unwrap();
    value
}
