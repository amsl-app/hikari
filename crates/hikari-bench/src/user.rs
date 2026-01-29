use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
}

impl From<&User> for hikari::User {
    fn from(user: &User) -> Self {
        Self {
            username: user.username.clone(),
            password: user.password.clone(),
        }
    }
}

impl From<User> for hikari::User {
    fn from(user: User) -> Self {
        Self {
            username: user.username,
            password: user.password,
        }
    }
}
