pub mod assessment;
pub mod history;
pub mod journal;
pub mod llm;
pub mod module;
pub mod quiz;
pub mod user;
pub mod user_handle;

pub trait IntoDbModel<T>: Sized {
    fn into_db_model(self) -> T;
}
pub trait FromDbModel<T>: Sized {
    fn from_db_model(model: T) -> Self;
}

pub trait IntoModel<T>: Sized {
    fn into_model(self) -> T;
}

pub trait FromModel<T>: Sized {
    fn from_model(model: T) -> Self;
}

impl<T, U> IntoModel<U> for T
where
    U: FromDbModel<T>,
{
    fn into_model(self) -> U {
        U::from_db_model(self)
    }
}

impl<T, U> IntoDbModel<U> for T
where
    U: FromModel<T>,
{
    fn into_db_model(self) -> U {
        U::from_model(self)
    }
}

pub trait TryIntoDbModel<T>: Sized {
    type Error;

    fn try_into_db_model(self) -> Result<T, Self::Error>;
}
pub trait TryFromDbModel<T>: Sized {
    type Error;

    fn try_from_db_model(model: T) -> Result<Self, Self::Error>;
}

pub trait TryIntoModel<T>: Sized {
    type Error;

    fn try_into_model(self) -> Result<T, Self::Error>;
}

pub trait TryFromModel<T>: Sized {
    type Error;

    fn try_from_model(model: T) -> Result<Self, Self::Error>;
}

impl<T, U> TryIntoModel<U> for T
where
    U: TryFromDbModel<T>,
{
    type Error = U::Error;

    fn try_into_model(self) -> Result<U, U::Error> {
        U::try_from_db_model(self)
    }
}

impl<T, U> TryIntoDbModel<U> for T
where
    U: TryFromModel<T>,
{
    type Error = U::Error;

    fn try_into_db_model(self) -> Result<U, U::Error> {
        U::try_from_model(self)
    }
}
