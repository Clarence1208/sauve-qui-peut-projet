use crate::models::Direction;
use serde::{Deserialize, Serialize};

/**
 * The RegisterTeam struct represents the content of the RegisterTeam message.
 * It contains the team name.
 */

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RegisterTeam {
    pub(crate) name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct SubscribePlayer {
    pub(crate) name: String,
    pub(crate) registration_token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Action {
    pub(crate) MoveTo: Direction,
}

/**
 * The message enum represents the different types of messages that can be sent to the server.
 * Each message type is represented by a struct.
 */
#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum Message {
    #[serde(rename_all = "camelCase")]
    RegisterTeam(RegisterTeam),
    SubscribePlayer(SubscribePlayer),
    Action(Action),
}
