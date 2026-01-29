use chrono::NaiveDate;
use sea_orm::entity::prelude::*;

#[derive(Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Clone, Copy)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(255))")]
pub enum Gender {
    #[sea_orm(string_value = "OTHER")]
    Other,
    #[sea_orm(string_value = "MALE")]
    Male,
    #[sea_orm(string_value = "FEMALE")]
    Female,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: Option<String>,
    pub birthday: Option<NaiveDate>,
    pub subject: Option<String>,
    pub semester: Option<i16>,
    pub gender: Option<Gender>,
    pub current_module: Option<String>,
    pub current_session: Option<String>,
    pub onboarding: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::config::Entity")]
    Config,
    #[sea_orm(has_one = "super::access_tokens::Entity")]
    AccessToken,
    #[sea_orm(has_many = "super::oidc_mapping::Entity")]
    OidcMapping,
    #[sea_orm(has_many = "super::oidc_groups::Entity")]
    OidcGroup,
    #[sea_orm(has_many = "super::custom_groups::Entity")]
    CustomGroup,
    #[sea_orm(has_many = "super::groups_token::Entity")]
    GroupToken,
    #[sea_orm(has_many = "super::quiz::score::Entity")]
    QuizScore,
}

impl Related<super::config::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Config.def()
    }
}

impl Related<super::access_tokens::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AccessToken.def()
    }
}

impl Related<super::oidc_mapping::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OidcMapping.def()
    }
}

impl Related<super::oidc_groups::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OidcGroup.def()
    }
}
impl Related<super::custom_groups::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CustomGroup.def()
    }
}

impl Related<super::groups_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GroupToken.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
