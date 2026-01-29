pub trait ItemId {
    type IdType;

    fn id(&self) -> Self::IdType;
}

#[allow(clippy::module_inception)]
pub mod id_map {
    use super::ItemId;
    use serde::Serialize;
    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<'a, S, T: ItemId + Serialize + 'a, I: IntoIterator<Item = (&'a T::IdType, &'a T)>>(
        map: I,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let iter = map.into_iter();
        serializer.collect_seq(iter.map(|(_, v)| v))
    }

    pub fn deserialize<'de, D, T: ItemId + Deserialize<'de>, O: FromIterator<(T::IdType, T)>>(
        deserializer: D,
    ) -> Result<O, D::Error>
    where
        D: Deserializer<'de>,
    {
        let elements = Vec::<T>::deserialize(deserializer)?;
        let map = elements.into_iter().map(|v| (v.id(), v)).collect();
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct TestStruct {
        #[serde(with = "id_map")]
        map: IndexMap<u64, TestItem>,
    }

    #[derive(Serialize, Deserialize)]
    struct TestItem {
        id: u32,
        value: u32,
    }

    impl ItemId for TestItem {
        type IdType = u64;

        fn id(&self) -> Self::IdType {
            u64::from(self.id)
        }
    }

    #[test]
    fn test_id_map() {
        let test_struct: TestStruct = serde_json::from_str(
            r#"{
            "map": [
                {"id": 1, "value": 2},
                {"id": 3, "value": 4}
            ]
        }"#,
        )
        .unwrap();
        assert_eq!(test_struct.map.len(), 2);
        let data = test_struct.map.into_iter().collect::<Vec<_>>();
        assert!(matches!(
            data.as_slice(),
            [(1, TestItem { id: 1, value: 2 }), (3, TestItem { id: 3, value: 4 })]
        ));
    }

    #[test]
    fn test_id_map_serialize() {
        let map = IndexMap::from([(1, TestItem { id: 1, value: 2 }), (3, TestItem { id: 3, value: 4 })]);
        let test_struct = TestStruct { map };
        let json = serde_json::to_string(&test_struct).unwrap();
        assert_eq!(json, r#"{"map":[{"id":1,"value":2},{"id":3,"value":4}]}"#);
    }
}
