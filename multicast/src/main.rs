use clap::{value_t, App, Arg};
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

fn main() -> io::Result<()> {
    let matches = App::new("multicast")
        .version("0.1")
        .author("Dennis Walter <dennis.walter@gmail.com>")
        .about("Discover other hosts on the same (local) network.")
        .arg(
            Arg::with_name("host")
                .short("H")
                .long("host")
                .value_name("HOST")
                .default_value("127.0.0.1")
                .help("Hostname to listen on (server mode) or connect to (client mode)"),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .default_value("10101")
                .help("Port to listen on (server mode) or connect to (client mode)"),
        )
        .get_matches();
    let host = matches.value_of("host").unwrap();
    let port = value_t!(matches.value_of("port"), u16).unwrap_or_else(|e| e.exit());
    println!("Using host {}", host);
    println!("Using port {}", port);

    let multicast_addr: Ipv4Addr = IPV4_MULTICAST_ADDR
        .parse()
        .expect("Invalid IPv4 multicast address.");
    let _server_addr: Ipv4Addr = host.parse().expect("Invalid server address.");
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, IPV4_MULTICAST_PORT))?;
    socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED)?;
    socket.set_read_timeout(Some(Duration::from_millis(1000)))?;
    // Type of buf will be resolved to [u8; BUF_SIZE] later on through usage.
    let mut buf = [0; BUF_SIZE];
    let dsco_msg: Vec<u8> =
        bincode::serialize(&Message::Discover).expect("Cannot serialize Message.");
    socket.send_to(&dsco_msg, (multicast_addr, IPV4_MULTICAST_PORT))?;
    loop {
        match socket.recv_from(&mut buf) {
            Ok((n_bytes, src_addr)) => {
                println!("Received {} bytes from {}", n_bytes, src_addr);
                let reply: Message =
                    bincode::deserialize(&buf).expect("Could not deserialize message.");
                println!("Message is: {:?}", reply);
            }
            Err(e) => {
                if let io::ErrorKind::WouldBlock = e.kind() {
                    // Timeouts are OK.
                } else {
                    return Err(e);
                }
            }
        }
    }
    // Ok(())
}
