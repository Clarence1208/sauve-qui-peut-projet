use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;
use serde::Serialize;

/**
 * Send a message to the server
 *
 * @param stream: &mut TcpStream - The TCP stream to send the message
 * @param message: &Message - The message to send
 * @return io::Result<()> - The result of the operation
 */
pub fn send_message(stream: &mut TcpStream, message: &impl Serialize) -> io::Result<()> {
    // Log the preparation step
    println!("Preparing to send message...");

    // Serialize the message to JSON
    let serialized_message = serde_json::to_string(&message).expect("Failed to serialize message");
    println!("Serialized message: {}", serialized_message);

    // Send the message length (u32 in little-endian)
    let message_length = serialized_message.len() as u32;
    stream.write_all(&message_length.to_le_bytes()).map_err(|e| {
        eprintln!("Failed to send message length: {}", e);
        e
    })?;
    println!("Sent message length: {}", message_length);

    // Send the JSON message
    stream.write_all(serialized_message.as_bytes()).map_err(|e| {
        eprintln!("Failed to send message payload: {}", e);
        e
    })?;
    println!("Message sent successfully: {}", serialized_message);

    Ok(())
}



pub fn receive_message(stream: &mut TcpStream) -> io::Result<String> {
    // Read the length of the incoming message
    let mut length_buffer = [0; 4];
    stream.read_exact(&mut length_buffer).map_err(|_| {
        io::Error::new(io::ErrorKind::UnexpectedEof, "Failed to read message length")
    })?;
    let message_length = u32::from_le_bytes(length_buffer) as usize;
    println!("Received message length: {}", message_length);

    let mut message_buffer = Vec::with_capacity(message_length);
    let mut total_read = 0;

    while total_read < message_length {
        let mut chunk = vec![0; message_length - total_read];
        let bytes_read = stream.read(&mut chunk)?;
        if bytes_read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Connection closed before full message was received",
            ));
        }
        total_read += bytes_read;
        message_buffer.extend_from_slice(&chunk[..bytes_read]);
    }

    String::from_utf8(message_buffer)
        .map(|msg| msg.trim_matches(char::from(0)).to_string())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 message received"))
}


pub fn parse_token_from_response(response: &str) -> io::Result<String> {
    let registration_result: serde_json::Value =
        serde_json::from_str(response).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Failed to parse server response"))?;

    registration_result["RegisterTeamResult"]["Ok"]["registration_token"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Registration token not found"))
}