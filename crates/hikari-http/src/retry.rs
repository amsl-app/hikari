use futures_retry_policies::ShouldRetry;

pub enum MaybeRetry<T> {
    MaybeRetry(T),
    NoRetry(T),
}

impl<T> MaybeRetry<T> {
    pub fn into_inner(self) -> T {
        match self {
            Self::MaybeRetry(inner) | Self::NoRetry(inner) => inner,
        }
    }
}

impl<T> ShouldRetry for MaybeRetry<T> {
    fn should_retry(&self, _: u32) -> bool {
        match self {
            Self::MaybeRetry(_) => true,
            Self::NoRetry(_) => false,
        }
    }
}
