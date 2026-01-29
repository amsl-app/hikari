use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Arg, Command, Error};
use std::ffi::OsStr;

#[derive(Debug, Clone)]
pub(crate) struct NamedOptionalValue {
    pub name: String,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub(crate) struct NamedOptionalValueParser;

impl clap::builder::TypedValueParser for NamedOptionalValueParser {
    type Value = NamedOptionalValue;

    fn parse_ref(&self, cmd: &Command, _: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
        let value = value
            .to_str()
            .ok_or_else(|| Error::new(ErrorKind::InvalidUtf8).with_cmd(cmd))?;
        if let Some((key, value)) = value.split_once('=') {
            return Ok(NamedOptionalValue {
                name: key.trim().to_owned(),
                value: serde_json::from_str(value.trim()).map_err(|error| {
                    let mut err = Error::new(ErrorKind::InvalidValue).with_cmd(cmd);
                    err.insert(ContextKind::InvalidValue, ContextValue::String(value.trim().to_owned()));
                    err.insert(ContextKind::Custom, ContextValue::String(error.to_string()));
                    err
                })?,
            });
        }
        Ok(NamedOptionalValue {
            name: value.trim().to_owned(),
            value: None,
        })
    }
}
