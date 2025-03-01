use std::net::TcpListener;
use log::{info, error};

fn main() {

//Lancer server sur port
//Register player
//Answer to Messages


    env_logger::init();
    info!("Starting server on port 8778");
    let listener = TcpListener::bind("127.0.0.1:8778");

    for client in listener.incoming() {

        match client {
            Ok(client) => {
                println!("Hello world");
            }
            Err(error) => {
                error!("Failed to accept client connection: {}", error);
            }
        }
    }

}
