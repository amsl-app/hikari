mod postgresql;
mod sqlite;
pub mod util;

pub use postgresql::*;
pub use sqlite::*;
use std::borrow::Cow;

pub trait TestDb {
    fn db_uri(&self) -> Cow<'_, str>;
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
