use crate::error::{Error, NetworkError, ProtocolError};
use crate::logger::log_message;
use serde::Serialize;
use std::io::{Read, Write};
use std::net::TcpStream;

const LOG_MESSAGE_CATEGORY: &str = "server_message";

///Send a message to the server
///
/// @param stream: &mut TcpStream - The TCP stream to send the message
/// @param message: &Message - The message to send
/// @return io::Result<()> - The result of the operation
pub fn send_message(stream: &mut TcpStream, message: &impl Serialize) -> Result<(), Error> {
    // Log the preparation step
    log_message(LOG_MESSAGE_CATEGORY, "Preparing to send message...")?;

    // Serialize the message to JSON
    let serialized_message =
        serde_json::to_string(&message).map_err(|_| ProtocolError::SerializationFailed)?;
    log_message(
        LOG_MESSAGE_CATEGORY,
        &format!("Serialized message: {}", serialized_message),
    )?;

    // Send the message length (u32 in little-endian)
    let message_length = serialized_message.len() as u32;
    stream
        .write_all(&message_length.to_le_bytes())
        .map_err(|_| NetworkError::SendLengthFailed)?;
    log_message(
        LOG_MESSAGE_CATEGORY,
        &format!("Sent message length: {}", message_length),
    )?;

    // Send the JSON message
    stream
        .write_all(serialized_message.as_bytes())
        .map_err(|_| NetworkError::SendPayloadFailed)?;
    log_message(LOG_MESSAGE_CATEGORY, "Message sent successfully.")?;

    Ok(())
}

pub fn receive_message(stream: &mut TcpStream) -> Result<String, Error> {
    // Read the length of the incoming message
    let mut length_buffer = [0; 4];
    stream
        .read_exact(&mut length_buffer)
        .map_err(|_| NetworkError::ReadLengthFailed)?;
    let message_length = u32::from_le_bytes(length_buffer) as usize;
    log_message(
        LOG_MESSAGE_CATEGORY,
        &format!("Received message length: {}", message_length),
    )?;

    // Now read the message itself
    let mut message_buffer = vec![0; message_length];
    let mut total_read = 0;

    while total_read < message_length {
        match stream.read(&mut message_buffer[total_read..]) {
            Ok(0) => {
                return Err(NetworkError::ReadPayloadFailed.into());
            }
            Ok(n) => {
                total_read += n;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return Err(NetworkError::ReadPayloadFailed.into()),
        }
    }

    let message =
        String::from_utf8(message_buffer).map_err(|_| NetworkError::Utf8ConversionFailed)?;

    Ok(message)
}

pub fn parse_token_from_response(response: &str) -> Result<String, Error> {
    let registration_result: serde_json::Value =
        serde_json::from_str(response).map_err(|_| ProtocolError::ResponseParsingFailed)?;

    registration_result["RegisterTeamResult"]["Ok"]["registration_token"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| ProtocolError::TokenNotFound.into())
}
