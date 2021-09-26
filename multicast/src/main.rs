use clap::{value_t, App, Arg};
use std::io;
use std::net::{Ipv4Addr, UdpSocket};

const IPV4_MULTICAST_ADDR: &'static str = "224.0.0.199";
const IPV4_MULTICAST_PORT: u16 = 10199;

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
    // Type of buf will be resolved to [u8; 32] later on through usage.
    let mut buf = [0; 1024];
    loop {
        let (n_bytes, src_addr) = socket.recv_from(&mut buf)?;
        println!("Received {} bytes from {}", n_bytes, src_addr);
    }
     // Ok(())
}
