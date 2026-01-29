use crate::util::ValidatableTable;
use sea_schema::postgres::def::{ColumnInfo as PgColumnInfo, TableDef as PgTableDef};
use std::collections::HashMap;

pub trait ValidatePg {
    #[must_use]
    fn build_pg_column_map<'map, 'a: 'map, 'b>(
        tables: &'map HashMap<&'a str, &'a PgTableDef>,
        table: &'b str,
    ) -> HashMap<&'a str, &'a PgColumnInfo> {
        let table = *tables.get(table).unwrap();
        let columns: HashMap<_, _> = table
            .columns
            .iter()
            .map(|column_info| (column_info.name.as_str(), column_info))
            .collect();
        columns
    }
    fn validate_pg<'map, 'a: 'map>(tables: &'map HashMap<&'a str, &'a PgTableDef>);
}

impl<T> ValidatePg for T
where
    T: for<'a> ValidatableTable<&'a PgColumnInfo>,
{
    fn validate_pg<'map, 'a: 'map>(tables: &'map HashMap<&'a str, &'a PgTableDef>) {
        let map = Self::build_pg_column_map(tables, Self::name());
        assert_eq!(map.len(), Self::size());
        Self::validate(&map);
    }
}

#[macro_export]
macro_rules! postgres_assert_type_base {
    ($db_type: pat, $columns:ident, $name: literal, $not_null: literal) => {
        assert!(matches!($columns.get($name).unwrap().col_type, $db_type));
        assert_eq!($columns.get($name).unwrap().not_null.is_some(), $not_null);
    };

    ($db_type: pat, $columns:ident, $name: literal) => {
        $crate::postgres_assert_type_base!($db_type, $columns, $name, true);
    };
}

#[macro_export]
macro_rules! postgres_assert_uuid {
    ($columns:ident, $name: literal $(, $tail:tt)*) => {
        $crate::postgres_assert_type_base!(
            sea_schema::postgres::def::ColumnType::Uuid,
            $columns,
            $name
            $(, $tail)*
        );
    };
}

pub use postgres_assert_uuid as assert_uuid;

#[macro_export]
macro_rules! postgres_assert_string {
    ($columns:ident, $name: literal $(, $tail:tt)*) => {
        $crate::postgres_assert_type_base!(
            sea_schema::postgres::def::ColumnType::Varchar(sea_schema::postgres::def::StringAttr {
                length: None
            }),
            $columns,
            $name
            $(, $tail)*
        );
    };
}

pub use postgres_assert_string as assert_string;

#[macro_export]
macro_rules! postgres_assert_timestamp {
    ($columns:ident, $name: literal $(, $tail:tt)*) => {
        $crate::postgres_assert_type_base!(
            sea_schema::postgres::def::ColumnType::Timestamp(sea_schema::postgres::def::TimeAttr {
                precision: Some(6)
            }),
            $columns,
            $name
            $(, $tail)*
        );
    };
}

pub use postgres_assert_timestamp as assert_timestamp;

#[macro_export]
macro_rules! postgres_assert_integer {
    ($columns:ident, $name: literal $(, $tail:tt)*) => {
        $crate::postgres_assert_type_base!(
            sea_schema::postgres::def::ColumnType::Integer,
            $columns,
            $name
            $(, $tail)*
        );
    };
}

pub use postgres_assert_integer as assert_integer;
