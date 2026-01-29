use chrono::NaiveDateTime;

use hikari_entity::llm::message::Direction as DirectionModel;
use hikari_entity::llm::message::Model;
use hikari_entity::llm::message::Status as MessageStatusModel;
use hikari_entity::llm::message::{ContentType as ContentTypeModel, ContentType};
use hikari_model::chat::{Direction, TypeSafePayload};
use hikari_model::llm::message::{ConversationMessage, MessageStatus};

use crate::convert::{FromDbModel, FromModel, IntoDbModel, IntoModel};

impl FromDbModel<MessageStatusModel> for MessageStatus {
    fn from_db_model(model: MessageStatusModel) -> Self {
        match model {
            MessageStatusModel::Generating => Self::Generating,
            MessageStatusModel::Completed => Self::Completed,
        }
    }
}

impl FromModel<MessageStatus> for MessageStatusModel {
    fn from_model(model: MessageStatus) -> Self {
        match model {
            MessageStatus::Generating => Self::Generating,
            MessageStatus::Completed => Self::Completed,
        }
    }
}

impl FromDbModel<DirectionModel> for Direction {
    fn from_db_model(model: DirectionModel) -> Self {
        match model {
            DirectionModel::Send => Self::Send,
            DirectionModel::Receive => Self::Receive,
        }
    }
}

impl FromModel<Direction> for DirectionModel {
    fn from_model(model: Direction) -> Self {
        match model {
            Direction::Send => Self::Send,
            Direction::Receive => Self::Receive,
        }
    }
}

impl FromDbModel<Model> for ConversationMessage {
    fn from_db_model(model: Model) -> Self {
        Self {
            conversation_id: model.conversation_id,
            message_order: model.message_order,
            message: match model.content_type {
                // TODO safe unwrap
                ContentTypeModel::Text => {
                    TypeSafePayload::Text(serde_json::from_str(&model.payload).expect("failed to decode text payload"))
                }
                ContentTypeModel::Payload => {
                    TypeSafePayload::Payload(serde_json::from_str(&model.payload).expect("failed to decode payload"))
                }
                ContentTypeModel::Buttons => TypeSafePayload::Button(
                    serde_json::from_str(&model.payload).expect("failed to decode button payload"),
                ),
            },
            step: model.step,
            direction: model.direction.into_model(),
            status: model.status.into_model(),
        }
    }
}

impl FromModel<ConversationMessage> for Model {
    fn from_model(model: ConversationMessage) -> Self {
        Self {
            conversation_id: model.conversation_id,
            message_order: model.message_order,
            step: model.step,
            created_at: NaiveDateTime::default(),
            content_type: match &model.message {
                TypeSafePayload::Text(_) => ContentTypeModel::Text,
                TypeSafePayload::Button(_) => ContentTypeModel::Buttons,
                _ => ContentTypeModel::Payload,
            },
            payload: serde_json::to_string(&model.message).expect("failed to decode message payload"),
            direction: model.direction.into_db_model(),
            status: model.status.into_db_model(),
        }
    }
}

pub fn split_payload_for_database(payload: TypeSafePayload) -> Result<(ContentType, String), anyhow::Error> {
    match payload {
        TypeSafePayload::Text(text) => {
            let text_str = serde_json::to_string(&text)?;
            Ok((ContentType::Text, text_str))
        }
        TypeSafePayload::Button(button) => {
            let button_str = serde_json::to_string(&button)?;
            Ok((ContentType::Buttons, button_str))
        }
        TypeSafePayload::Payload(payload) => {
            let payload_str = serde_json::to_string(&payload)?;
            Ok((ContentType::Payload, payload_str))
        }
        TypeSafePayload::FlowTrigger(_flow_trigger) => {
            Err(anyhow::Error::msg("FlowTrigger payload type not supported".to_owned()))
        }
    }
}
