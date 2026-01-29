use std::collections::HashMap;

use serde::Deserialize;
use serde_yml::Value;

use crate::constants::v01::collection::ConstantCollectionV01;

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ConstantCollection {
    pub constants: HashMap<String, Value>,
}

impl From<ConstantCollectionV01> for ConstantCollection {
    fn from(value: ConstantCollectionV01) -> Self {
        let mut constants = HashMap::new();
        for constant in value.constants {
            constants.insert(constant.name, constant.value);
        }
        ConstantCollection { constants }
    }
}
