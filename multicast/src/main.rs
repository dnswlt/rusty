use clap::{App, Arg};
use serde::{Deserialize, Serialize};
use std::io;
use std::net::{Ipv4Addr, UdpSocket};
use std::time::Duration;

const IPV4_MULTICAST_ADDR: &'static str = "224.0.0.199";
const IPV4_MULTICAST_PORT: u16 = 10199;
const BUF_SIZE: usize = 1024;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Message {
    Discover,
    Hello,
}

fn server(multicast_addr: Ipv4Addr, multicast_port: u16) -> io::Result<()> {
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
                        let reply =
                            bincode::serialize(&Message::Hello).expect("Cannot serialize Message.");
                        socket.send_to(&reply, &src_addr)?;
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

fn client(multicast_addr: Ipv4Addr, multicast_port: u16) -> io::Result<()> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    let dsco_msg = bincode::serialize(&Message::Discover).expect("Cannot serialize Message.");
    socket.set_read_timeout(Some(Duration::from_millis(2000)))?;
    for i in 0..10 {
        socket.send_to(&dsco_msg, (multicast_addr, multicast_port))?;
    }
    loop {
        let mut buf = [0; BUF_SIZE];
        match socket.recv_from(&mut buf) {
            Ok((_, src_addr)) => match bincode::deserialize(&buf) {
                Ok(Message::Hello) => {
                    println!("Received reply from {}", src_addr);
                }
                _ => {
                    println!("Ignoring invalid message.");
                }
            },
            Err(e) => {
                if let io::ErrorKind::WouldBlock = e.kind() {
                    return Ok(());
                } else {
                    return Err(e);
                }
            }
        }
    }
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
        .get_matches();
    let multicast_addr: Ipv4Addr = IPV4_MULTICAST_ADDR
        .parse()
        .expect("Invalid IPv4 multicast address.");
    if matches.is_present("server_mode") {
        server(multicast_addr, IPV4_MULTICAST_PORT)
    } else {
        client(multicast_addr, IPV4_MULTICAST_PORT)
    }
}
