use std::fmt;

#[derive(Debug, PartialEq)]
pub enum NetworkError {
    ConnectionFailed,
    SendLengthFailed,
    SendPayloadFailed,
    ReadLengthFailed,
    ReadPayloadFailed,
    Utf8ConversionFailed,
}

#[derive(Debug, PartialEq)]
pub enum ProtocolError {
    SerializationFailed,
    ResponseParsingFailed,
    TokenNotFound,
    InvalidArguments,
    InvalidAddressFormat,
    RegistrationFailed,
}

#[derive(Debug, PartialEq)]
pub enum LogError {
    DirectoryCreationFailed,
    FileOpenFailed,
    MetadataFailed,
    WriteFailed,
    MutexPoisoned,
}

#[derive(Debug, PartialEq)]
pub enum DecodeError {
    InvalidSize,
    UnauthorizedCharacter(char),
    InvalidSegmentSize,
}

#[derive(Debug, PartialEq)]
pub enum PlayerError {
    SubscriptionFailed(String),
    ActionFailed,
    RadarResponseFailed,
    HintHandlingFailed,
    ChallengeResolutionFailed,
    InvalidRadarData,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::ConnectionFailed => write!(f, "Failed to connect to server"),
            NetworkError::SendLengthFailed => write!(f, "Failed to send message length"),
            NetworkError::SendPayloadFailed => write!(f, "Failed to send message payload"),
            NetworkError::ReadLengthFailed => write!(f, "Failed to read message length"),
            NetworkError::ReadPayloadFailed => write!(f, "Failed to read message payload"),
            NetworkError::Utf8ConversionFailed => write!(f, "Invalid UTF-8 message received"),
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::SerializationFailed => write!(f, "Failed to serialize message"),
            ProtocolError::ResponseParsingFailed => write!(f, "Failed to parse server response"),
            ProtocolError::TokenNotFound => write!(f, "Registration token not found"),
            ProtocolError::InvalidArguments => write!(f, "Usage: worker <server_address>"),
            ProtocolError::InvalidAddressFormat => {
                write!(f, "Invalid server address. Use <host:port> format")
            }
            ProtocolError::RegistrationFailed => write!(f, "Failed to register team"),
        }
    }
}

impl fmt::Display for LogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogError::DirectoryCreationFailed => write!(f, "Failed to create log directory"),
            LogError::FileOpenFailed => write!(f, "Failed to open log file"),
            LogError::MetadataFailed => write!(f, "Failed to retrieve file metadata"),
            LogError::WriteFailed => write!(f, "Failed to write to log file"),
            LogError::MutexPoisoned => write!(f, "Mutex poisoned"),
        }
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::InvalidSize => write!(f, "Invalid size (form 4n+1)"),
            DecodeError::UnauthorizedCharacter(c) => write!(f, "Character unauthorized '{}'", c),
            DecodeError::InvalidSegmentSize => {
                write!(f, "Segment size invalid (less than 2 characters)")
            }
        }
    }
}

impl fmt::Display for PlayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlayerError::SubscriptionFailed(msg) => {
                write!(f, "Failed to subscribe player: {}", msg)
            }
            PlayerError::ActionFailed => write!(f, "Failed to send action"),
            PlayerError::RadarResponseFailed => write!(f, "Failed to receive radar response"),
            PlayerError::HintHandlingFailed => write!(f, "Failed to handle hint"),
            PlayerError::ChallengeResolutionFailed => write!(f, "Failed to resolve challenge"),
            PlayerError::InvalidRadarData => write!(f, "Invalid radar data"),
        }
    }
}

impl std::error::Error for NetworkError {}
impl std::error::Error for ProtocolError {}
impl std::error::Error for LogError {}
impl std::error::Error for DecodeError {}
impl std::error::Error for PlayerError {}

// A common error type that encompasses all possible errors
#[derive(Debug, PartialEq)]
pub enum Error {
    Network(NetworkError),
    Protocol(ProtocolError),
    Log(LogError),
    Decode(DecodeError),
    Player(PlayerError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Network(e) => write!(f, "Network error: {}", e),
            Error::Protocol(e) => write!(f, "Protocol error: {}", e),
            Error::Log(e) => write!(f, "Log error: {}", e),
            Error::Decode(e) => write!(f, "Decode error: {}", e),
            Error::Player(e) => write!(f, "Player error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Network(e) => Some(e),
            Error::Protocol(e) => Some(e),
            Error::Log(e) => Some(e),
            Error::Decode(e) => Some(e),
            Error::Player(e) => Some(e),
        }
    }
}

// Implement From for each specific error type
impl From<NetworkError> for Error {
    fn from(err: NetworkError) -> Self {
        Error::Network(err)
    }
}

impl From<ProtocolError> for Error {
    fn from(err: ProtocolError) -> Self {
        Error::Protocol(err)
    }
}

impl From<LogError> for Error {
    fn from(err: LogError) -> Self {
        Error::Log(err)
    }
}

impl From<DecodeError> for Error {
    fn from(err: DecodeError) -> Self {
        Error::Decode(err)
    }
}

impl From<PlayerError> for Error {
    fn from(err: PlayerError) -> Self {
        Error::Player(err)
    }
}
