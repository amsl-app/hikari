use crate::util::ValidatableTable;
use sea_schema::sqlite::def::{ColumnInfo as SqliteColumnInfo, TableDef as SqliteTableDef};
use std::collections::HashMap;
pub extern crate sea_query;

pub trait ValidateSqlite {
    #[must_use]
    fn build_sqlite_column_map<'map, 'a: 'map, 'b>(
        tables: &'map HashMap<&'a str, &'a SqliteTableDef>,
        table: &'b str,
    ) -> HashMap<&'a str, &'a SqliteColumnInfo> {
        let table = *tables.get(table).unwrap();
        let columns: HashMap<_, _> = table.columns.iter().map(|cdef| (cdef.name.as_str(), cdef)).collect();
        columns
    }
    fn validate_sqlite<'map, 'a: 'map>(tables: &'map HashMap<&'a str, &'a SqliteTableDef>);
}

impl<T> ValidateSqlite for T
where
    T: for<'a> ValidatableTable<&'a SqliteColumnInfo>,
{
    fn validate_sqlite<'map, 'a: 'map>(tables: &'map HashMap<&'a str, &'a SqliteTableDef>) {
        let map = Self::build_sqlite_column_map(tables, Self::name());
        assert_eq!(map.len(), Self::size());
        Self::validate(&map);
    }
}

#[macro_export]
macro_rules! sqlite_assert_type_base {
    ($db_type: pat, $columns:ident, $name: literal, $not_null: literal) => {
        assert!(
            matches!($columns.get($name).unwrap().r#type, $db_type),
            "expected type {}. actual type {:?}",
            stringify!($db_type),
            $columns.get($name).unwrap().r#type
        );
        assert_eq!($columns.get($name).unwrap().not_null, $not_null);
    };

    ($db_type: pat, $columns:ident, $name: literal) => {
        $crate::sqlite_assert_type_base!($db_type, $columns, $name, true);
    };
}

#[macro_export]
macro_rules! sqlite_assert_text {
    ($columns:ident, $name: literal $(, $tail:tt)*) => {
        $crate::sqlite_assert_type_base!(
            $crate::util::sqlite::sea_query::ColumnType::String(_),
            $columns,
            $name
            $(, $tail)*
        );
    };
}

#[macro_export]
macro_rules! sqlite_assert_uuid {
    ($columns:ident, $name: literal $(, $tail:tt)*) => {
        $crate::sqlite_assert_type_base!(
            $crate::util::sqlite::sea_query::ColumnType::Uuid,
            $columns,
            $name
            $(, $tail)*
        );
    };
}

#[macro_export]
macro_rules! sqlite_assert_timestamp {
    ($columns:ident, $name: literal $(, $tail:tt)*) => {
        $crate::sqlite_assert_type_base!(
            $crate::util::sqlite::sea_query::ColumnType::Timestamp,
            $columns,
            $name
            $(, $tail)*
        );
    };
}

pub use sqlite_assert_text as assert_text;
pub use sqlite_assert_timestamp as assert_timestamp;
pub use sqlite_assert_uuid as assert_uuid;

#[macro_export]
macro_rules!sqlite_assert_integer {
    ($columns:ident, $name: literal $(, $tail:tt)*) => {
        $crate::sqlite_assert_type_base!(
            $crate::util::sqlite::sea_query::ColumnType::Integer,
            $columns,
            $name
            $(, $tail)*
        );
    };
}

pub use sqlite_assert_integer as assert_integer;
