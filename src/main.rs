extern crate mio;

use mio::net::{TcpListener, TcpStream};
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio::unix::UnixReady;

use std::collections::HashMap;
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::str;

const ANONYMOUS: bool = false;

const SERVER: Token = Token(0);

pub struct TcpConnection {
    pub sock: TcpStream,
    pub addr: SocketAddr,
}

impl TcpConnection {
    pub fn new(sock: TcpStream, addr: SocketAddr) -> TcpConnection {
        TcpConnection { sock, addr }
    }
}

fn main() {
    let address = "127.0.0.1:4321";
    let server_addr: SocketAddr = address.parse().unwrap();
    let server = TcpListener::bind(&server_addr).unwrap();
    let poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1024);

    poll.register(&server, SERVER, Ready::readable(), PollOpt::edge())
        .unwrap();

    println!("The server has been started at {}", address);

    let mut conns: HashMap<Token, TcpConnection> = HashMap::with_capacity(64);
    let mut next_token: usize = 1;

    loop {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            let (token, kind) = (event.token(), event.readiness());

            if UnixReady::from(kind).is_error() || UnixReady::from(kind).is_hup() {
                let conn = conns.remove(&token).unwrap();
                println!("Client disconnected: {}, token: {}", conn.addr, token.0);
                println!("Currently connected: {}", conns.len());
                continue;
            }

            if token == SERVER {
                let (sock, addr) = match server.accept() {
                    Ok((s, addr)) => (s, addr),
                    Err(e) => {
                        if e.kind() != ErrorKind::WouldBlock {
                            println!("Error");
                        }
                        // spurious events should not be treated as errors
                        return;
                    }
                };
                poll.register(
                    &sock,
                    Token(next_token),
                    Ready::readable() | UnixReady::hup(),
                    PollOpt::edge(),
                ).unwrap();
                conns.insert(Token(next_token), TcpConnection::new(sock, addr));
                println!(
                    "New connection was accepted: {}, token: {}",
                    addr, next_token
                );
                next_token += 1;
                println!("Currently connected: {}", conns.len());
            } else {
                let mut buf = vec![];
                let conn = conns
                    .get_mut(&token)
                    .unwrap();
                match conn.sock.read_to_end(&mut buf) {
                    Ok(_) => break,
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) => panic!("IO error: {}", e),
                };
                let sender_address = conn.addr.to_string();
                print!("New message from {}: {}",
                    sender_address, str::from_utf8(&buf).unwrap());
                
                for (token_key, conn) in &mut conns {
                    if token_key.0 != token.0 {
                        if ANONYMOUS == true {
                            conn.sock.write_all(&buf).unwrap();
                        } else {
                            let response = sender_address.to_owned() + ": " + str::from_utf8(&buf).unwrap();
                            conn.sock.write_all(&response.into_bytes()).unwrap();
                        }
                    }
                }
            }
        }
    }
}
