use crate::convert::FromDbModel;
use hikari_entity::tag::Model as TagModel;
use hikari_model::tag::Tag;

impl FromDbModel<TagModel> for Tag {
    fn from_db_model(model: TagModel) -> Self {
        Tag {
            id: model.id,
            name: model.name,
            user_id: model.user_id,
            icon: model.icon,
            hidden: model.hidden,
        }
    }
}
