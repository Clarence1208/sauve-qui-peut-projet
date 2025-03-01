use serde::{Deserialize, Serialize};
use crate::{RegisterTeamResult, RelativeCompass, SubscribePlayerResult};

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
pub(crate) struct Answer {
    pub(crate) answer: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) enum Action {
    MoveTo(Direction),
    SolveChallenge(Answer),
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

// Direction enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) enum Direction {
    Front,
    Back,
    Left,
    Right,
}

// Response models
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RegisterTeamResponseOk {
    pub(crate) expected_players: usize,
    pub(crate) registration_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum RegisterTeamResponseResult {
    Ok(RegisterTeamResponseOk),
    Error(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RegisterTeamResponse {
    pub(crate) RegisterTeamResult: RegisterTeamResponseResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum SubscribePlayerResponseResult {
    Ok,
    Error(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SubscribePlayerResponse {
    pub(crate) SubscribePlayerResult: SubscribePlayerResponseResult,
}

// New response type for radar view
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RadarViewResponse {
    pub(crate) RadarView: String,
}

// New response type for found exit
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct FoundExitResponse {
    pub(crate) FoundExit: bool,
}

// New response type for wall notification
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CannotPassThroughWallResponse {
    pub(crate) CannotPassThroughWall: bool,
}

// New response type for hint with compass
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CompassData {
    pub(crate) angle: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RelativeCompassResponse {
    pub(crate) RelativeCompass: CompassData,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct HintResponse {
    pub(crate) Hint: RelativeCompassResponse,
}

// Message types to client
// #[derive(Debug, Serialize)]
// #[serde(tag = "type", rename_all = "camelCase")]
// enum ServerMessage {
//     RegisterTeamResult(RegisterTeamResult),
//     SubscribePlayerResult(SubscribePlayerResult),
//     RadarView(String),
//     Hint(RelativeCompass),
//     FoundExit,
//     CannotPassThroughWall,
// }
