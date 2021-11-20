use chrono::Local;
use clap::{value_t, App, Arg};
use serde::{Deserialize, Serialize};
use std::io;
use std::net::{Ipv4Addr, UdpSocket};
use std::process::Command;
use std::str;
use std::time::Duration;

const IPV4_MULTICAST_ADDR: &'static str = "224.0.0.199";
const IPV4_MULTICAST_PORT: u16 = 10199;
const BUF_SIZE: usize = 1024;

#[derive(Serialize, Deserialize, Debug)]
struct ServerInfo {
    hostname: String,
    local_time: String,
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
enum Message {
    Discover,
    Hello(ServerInfo),
}

fn get_hostname() -> io::Result<String> {
    let output = Command::new("hostname").output()?;
    match str::from_utf8(&output.stdout) {
        Ok(h) => Ok(h.trim_end().to_string()), // Remove trailing "\n".
        Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
    }
}

fn server(multicast_addr: Ipv4Addr, multicast_port: u16, message: &str) -> io::Result<()> {
    // Type of buf will be resolved to [u8; BUF_SIZE] later on through usage.
    let mut buf = [0; BUF_SIZE];
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, multicast_port))?;
    socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED)?;
    loop {
        match socket.recv_from(&mut buf) {
            Ok((n_bytes, src_addr)) => {
                println!("Received {} bytes from {}", n_bytes, src_addr);
                match bincode::deserialize(&buf) {
                    Ok(Message::Discover) => {
                        let hostname = match get_hostname() {
                            Ok(h) => h,
                            Err(e) => {
                                eprintln!("Could not get hostname: {}", e);
                                String::from("")
                            }
                        };
                        let hello = Message::Hello(ServerInfo {
                            hostname: hostname,
                            local_time: Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
                            message: message.to_string(),
                        });
                        let server_msg =
                            bincode::serialize(&hello).expect("Cannot serialize Hello Message.");
                        socket.send_to(&server_msg, &src_addr)?;
                    }
                    _ => {
                        println!("Ignoring invalid message.");
                    }
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

fn client(multicast_addr: Ipv4Addr, multicast_port: u16, limit: i32) -> io::Result<()> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    let dsco_msg = bincode::serialize(&Message::Discover).expect("Cannot serialize Message.");
    socket.set_read_timeout(Some(Duration::from_millis(2000)))?;
    for _ in 0..limit {
        socket.send_to(&dsco_msg, (multicast_addr, multicast_port))?;
        loop {
            let mut buf = [0; BUF_SIZE];
            match socket.recv_from(&mut buf) {
                Ok((_, src_addr)) => match bincode::deserialize(&buf) {
                    Ok(Message::Hello(server_info)) => {
                        println!(
                            "Received reply from {} ({}, {}{})",
                            src_addr,
                            &server_info.hostname,
                            &server_info.local_time,
                            if server_info.message.is_empty() {
                                server_info.message
                            } else {
                                format!(" \"{}\"", &server_info.message)
                            }
                        );
                    }
                    _ => {
                        println!("Ignoring invalid message.");
                    }
                },
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => {
                        break;
                    }
                    _ => {
                        return Err(e);
                    }
                },
            }
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let matches = App::new("multicast")
        .version("0.1")
        .author("Dennis Walter <dennis.walter@gmail.com>")
        .about("Discover other hosts on the same (local) network.")
        .arg(
            Arg::with_name("server_mode")
                .short("s")
                .long("server")
                .help("Run in server mode."),
        )
        .arg(
            Arg::with_name("limit")
                .short("n")
                .long("limit")
                .default_value("1")
                .help("Number of discovery messages to send as client."),
        )
        .arg(
            Arg::with_name("message")
                .short("m")
                .long("message")
                .takes_value(true)
                .help("Optional message to send back in Hello messages."),
        )
        .get_matches();
    let multicast_addr: Ipv4Addr = IPV4_MULTICAST_ADDR
        .parse()
        .expect("Invalid IPv4 multicast address.");
    if matches.is_present("server_mode") {
        server(
            multicast_addr,
            IPV4_MULTICAST_PORT,
            matches.value_of("message").unwrap_or(""),
        )
    } else {
        let limit = value_t!(matches.value_of("limit"), i32).unwrap_or_else(|e| e.exit());
        client(multicast_addr, IPV4_MULTICAST_PORT, limit)
    }
}
