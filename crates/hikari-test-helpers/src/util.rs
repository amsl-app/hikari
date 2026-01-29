use std::collections::HashMap;
pub mod postgres;
pub mod sqlite;

pub trait TestableTable {
    fn name() -> &'static str;
    fn size() -> usize;
}

pub trait ValidatableTable<T>: TestableTable {
    fn validate(columns: &HashMap<&str, T>);
}

// source: https://danielkeep.github.io/tlborm/book/blk-counting.html
#[macro_export]
macro_rules! replace_expr {
    ($_p:path : $sub:expr_2021) => {
        $sub
    };
}

#[macro_export]
macro_rules! count_paths {
    ($($path:path),*) => {
        [$($crate::replace_expr!($path : ())),*].len()
    };
}

#[macro_export]
macro_rules! build_test_set {
    ($($table:ident$(::$rest:ident)*),+) => {
        fn validate_sqlite<'map, 'a: 'map>(tables: &'map HashMap<&'a str, &'a sea_schema::sqlite::def::TableDef>) {
            $(<$table$(::$rest)* as $crate::util::sqlite::ValidateSqlite>::validate_sqlite(&tables));+
        }
        fn validate_pg<'map, 'a: 'map>(tables: &'map HashMap<&'a str, &'a sea_schema::postgres::def::TableDef>) {
            $(<$table$(::$rest)* as $crate::util::postgres::ValidatePg>::validate_pg(&tables));+
        }
        const COUNT: usize = $crate::count_paths!($($table),+);
    };
}

pub use build_test_set;
