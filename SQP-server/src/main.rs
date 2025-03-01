use std::net::TcpListener;
use log::{info, error};
use std::io::{Read, Write};
use std::{thread};
fn main() {

    env_logger::init();

    match init_server() {
        Ok(_) => {
            info!("Started server on port 8778");
            println!("Started server on port 8778");
        }
        Err(_) => {
            error!("Server didn't start on port 8778");
            eprintln!("Server didn't start on port 8778");
        }
    }

}

fn init_server() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8778")?;

    for client in listener.incoming() {

        match client {
            Ok(client) => {
                thread::spawn(move || handle_client(client)); // Spawn thread to handle each client
                //handle_client(client);
            }
            Err(error) => {
                error!("Failed to accept client connection: {}", error);
                eprintln!("Failed to accept client connection: {}", error);
            }
        }
    }
    Ok(())
}

fn handle_client(mut client: std::net::TcpStream) {
    let test = "Hello from server!";
    info!("New connection: {}", client.peer_addr().unwrap());
    println!("New connection: {}", client.peer_addr().unwrap());

    let mut size_buffer = [0; 4];
    client.read_exact(&mut size_buffer).unwrap();
    let n = u32::from_be_bytes(size_buffer);
    let mut char_buffer = vec![0; n as usize];
    client.read_exact(&mut char_buffer).unwrap();
    let s = String::from_utf8(char_buffer).unwrap();

    info!("{}", s);
    println!("{}", s);

    //fixme
    send_message(&mut client, &test).unwrap();
}

//Answer to client
fn send_message(stream: &mut std::net::TcpStream, message: &str) -> std::io::Result<()> {
    let message = message.as_bytes();
    let n = message.len() as u32;
    stream.write_all(&n.to_be_bytes())?;
    stream.write_all(message)?;
    Ok(())
}
