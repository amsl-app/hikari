use csml_interpreter::data::{CsmlBot, CsmlFlow};
use hikari_model::chat::{BotInfo, Direction, FlowInfo, Message, Payload};
use std::collections::HashMap;
use std::ops::Deref;
use std::slice;

pub(crate) fn flow_info_from_csml(flow: &CsmlFlow) -> FlowInfo<'_> {
    FlowInfo {
        id: &flow.id,
        name: &flow.name,
    }
}

pub(crate) fn bot_info_from_csml(bot: &CsmlBot) -> BotInfo<'_> {
    BotInfo {
        id: &bot.id,
        name: &bot.name,
        flows: bot.flows.iter().map(flow_info_from_csml).collect(),
    }
}

fn payload_from_csml_payload(payload: csml_engine::data::models::Payload) -> Payload {
    Payload {
        content: payload.content,
        content_type: payload.content_type,
    }
}

fn direction_from_csml_direction(direction: csml_engine::data::models::Direction) -> Direction {
    match direction {
        csml_engine::data::models::Direction::Send => Direction::Send,
        csml_engine::data::models::Direction::Receive => Direction::Receive,
    }
}

pub fn message_from_csml_message(message: csml_engine::data::models::Message) -> Message<Payload> {
    Message {
        payload: payload_from_csml_payload(message.payload),
        direction: direction_from_csml_direction(message.direction),
    }
}

pub fn message_from_csml_message_data(message: csml_engine::data::models::MessageData) -> Message<Payload> {
    Message {
        payload: payload_from_csml_payload(message.payload),
        direction: direction_from_csml_direction(message.direction),
    }
}

#[derive(Debug)]
pub(crate) struct Bots {
    bots: Vec<CsmlBot>,
}

impl Bots {
    pub fn new(bots: Vec<CsmlBot>) -> Self {
        Self { bots }
    }

    pub fn find(&self, bot_id: &str) -> Option<&CsmlBot> {
        // Bot name is used interchangeably for id as that is the behavior of csml_engine
        self.bots.iter().find(|bot| bot.id == bot_id || bot.name == bot_id)
    }

    pub fn ids(&self) -> HashMap<&String, Vec<&String>> {
        self.bots
            .iter()
            .map(|bot| (&bot.id, bot.flows.iter().map(|flow| &flow.id).collect()))
            .collect()
    }
}

impl IntoIterator for Bots {
    type Item = CsmlBot;
    type IntoIter = std::vec::IntoIter<CsmlBot>;

    fn into_iter(self) -> Self::IntoIter {
        self.bots.into_iter()
    }
}

impl<'a> IntoIterator for &'a Bots {
    type Item = &'a CsmlBot;
    type IntoIter = slice::Iter<'a, CsmlBot>;

    fn into_iter(self) -> Self::IntoIter {
        self.bots.iter()
    }
}

impl Deref for Bots {
    type Target = [CsmlBot];

    fn deref(&self) -> &Self::Target {
        &self.bots
    }
}
