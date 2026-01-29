use std::collections::HashSet;

use crate::convert::{FromDbModel, IntoDbModel, TryFromDbModel};
use crate::error::Error;
use hikari_entity::custom_groups::Model as CustomGroupModel;
use hikari_entity::oidc_groups::Model as OidcGroupModel;
use hikari_entity::user::Gender as GenderModel;
use hikari_entity::user::Model as UserModel;
use hikari_model::user::Gender;
use hikari_model::user::User;
use num_traits::ToPrimitive;

impl FromDbModel<GenderModel> for Gender {
    fn from_db_model(model: GenderModel) -> Self {
        match model {
            GenderModel::Other => Self::Other,
            GenderModel::Male => Self::Male,
            GenderModel::Female => Self::Female,
        }
    }
}

impl IntoDbModel<GenderModel> for Gender {
    fn into_db_model(self) -> GenderModel {
        match self {
            Self::Other => GenderModel::Other,
            Self::Male => GenderModel::Male,
            Self::Female => GenderModel::Female,
        }
    }
}

impl TryFromDbModel<UserModel> for User {
    type Error = Error;

    fn try_from_db_model(model: UserModel) -> Result<Self, Self::Error> {
        Ok(Self {
            id: model.id,
            name: model.name,
            birthday: model.birthday,
            subject: model.subject,
            semester: model
                .semester
                .map(|s| s.to_u8().ok_or(Error::NumConversion))
                .transpose()?,
            gender: model.gender.map(FromDbModel::from_db_model),
            current_module: model.current_module,
            onboarding: model.onboarding,
            groups: vec![],
        })
    }
}

impl TryFromDbModel<(UserModel, Vec<OidcGroupModel>, Vec<CustomGroupModel>)> for User {
    type Error = Error;

    fn try_from_db_model(
        (model, oidc_groups, custom_groups): (UserModel, Vec<OidcGroupModel>, Vec<CustomGroupModel>),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id: model.id,
            name: model.name,
            birthday: model.birthday,
            subject: model.subject,
            semester: model
                .semester
                .map(|s| s.to_u8().ok_or(Error::NumConversion))
                .transpose()?,
            gender: model.gender.map(FromDbModel::from_db_model),
            current_module: model.current_module,
            onboarding: model.onboarding,
            groups: oidc_groups
                .into_iter()
                .map(|group| group.value)
                .chain(custom_groups.into_iter().map(|group| group.value))
                .collect(),
        })
    }
}

impl TryFromDbModel<(UserModel, HashSet<String>, Vec<String>)> for User {
    type Error = Error;

    fn try_from_db_model(
        (model, oidc_groups, custom_groups): (UserModel, HashSet<String>, Vec<String>),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id: model.id,
            name: model.name,
            birthday: model.birthday,
            subject: model.subject,
            semester: model
                .semester
                .map(|s| s.to_u8().ok_or(Error::NumConversion))
                .transpose()?,
            gender: model.gender.map(FromDbModel::from_db_model),
            current_module: model.current_module,
            onboarding: model.onboarding,
            groups: oidc_groups.into_iter().chain(custom_groups).collect(),
        })
    }
}
