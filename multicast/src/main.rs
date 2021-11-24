use chrono::Local;
use clap::{value_t, App, Arg};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::io;
use std::net::{Ipv4Addr, UdpSocket};
use std::path::Path;
use std::process::Command;
use std::str;
use std::time::{Duration, Instant};
use std::thread;

const IPV4_MULTICAST_ADDR: &'static str = "224.0.0.199";
const IPV4_MULTICAST_PORT: u16 = 10199;
const BUF_SIZE: usize = 4096;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MacAddr {
    interface: String,
    address: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerInfo {
    hostname: String,
    mac_addresses: Vec<MacAddr>,
    local_time: String,
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
enum Message {
    Discover,
    Hello(ServerInfo),
}

impl fmt::Display for ServerInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ServerInfo{{\n")?;
        if !self.hostname.is_empty() {
            writeln!(f, "  hostname: {}", self.hostname)?;
        }
        if !self.mac_addresses.is_empty() {
            writeln!(f, "  mac_addresses: {{")?;
            for mac_addr in &self.mac_addresses {
                writeln!(f, "    {}: {}", mac_addr.interface, mac_addr.address)?;
            }
            writeln!(f, "  }}")?;
        }
        if !self.local_time.is_empty() {
            writeln!(f, "  local_time: {}", self.local_time)?;
        }
        if !self.message.is_empty() {
            writeln!(f, "  message: {}", self.message)?;
        }
        write!(f, "}}")
    }
}

fn get_mac_addrs() -> io::Result<Vec<MacAddr>> {
    let net = Path::new("/sys/class/net");
    if !net.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Probably not on a Linux machine: no /sys/class/net found.",
        ));
    }
    let mut addrs = Vec::new();
    for entry in net.read_dir()? {
        if let Ok(entry) = entry {
            let path_buf = entry.path().join("address");
            if let Some(iface) = entry.file_name().to_str() {
                if iface == "lo" {
                    continue; // Ignore loopback
                }
                let addr = fs::read_to_string(path_buf)?;
                addrs.push(MacAddr {
                    interface: iface.to_string(),
                    address: addr.trim_end().to_string(),
                });
            }
        }
    }
    return Ok(addrs);
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
    let mac_addresses = match get_mac_addrs() {
        Ok(mac_addrs) => mac_addrs,
        Err(e) => {
            eprintln!("Cannot get MAC addresses: {}", e);
            vec![]
        }
    };
    loop {
        let (n_bytes, src_addr) = socket.recv_from(&mut buf)?;
        println!("Received {} bytes from {}", n_bytes, src_addr);
        if let Ok(Message::Discover) = bincode::deserialize(&buf) {
            let hostname = get_hostname().unwrap_or_else(|e| {
                eprintln!("Could not get hostname: {}", e);
                String::from("")
            });
            let hello = Message::Hello(ServerInfo {
                hostname: hostname,
                mac_addresses: mac_addresses.clone(),
                local_time: Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
                message: message.to_string(),
            });
            let server_msg = bincode::serialize(&hello).expect("Cannot serialize Hello Message.");
            socket.send_to(&server_msg, &src_addr)?;
        } else {
            eprintln!("Ignoring invalid message from {}.", src_addr);
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
                Ok((_, src_addr)) => {
                    if let Ok(Message::Hello(server_info)) = bincode::deserialize(&buf) {
                        println!("Received reply from {}:\n{}", src_addr, server_info);
                    } else {
                        println!("Ignoring invalid message from {}.", src_addr);
                    }
                }
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
        let message = matches.value_of("message").unwrap_or("");
        // Try for at most 1 minute to start the server. This can be useful at system
        // startup, where the network interfaces might not be fully functional when
        // this program is started.
        const MAX_STARTUP_DELAY_SECONDS: u64 = 60;
        let started = Instant::now();
        loop {
            println!(
                "Trying to start server at {}:{}",
                multicast_addr, IPV4_MULTICAST_PORT
            );
            match server(multicast_addr, IPV4_MULTICAST_PORT, message) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    eprintln!("Failed to start server: {}", e);
                    let elapsed = started.elapsed();
                    if elapsed.as_secs() > MAX_STARTUP_DELAY_SECONDS {
                        eprintln!(
                            "Failed to start server for {}s. Giving up.",
                            MAX_STARTUP_DELAY_SECONDS
                        );
                        return Err(e);
                    }
                    thread::sleep(Duration::from_millis(1000));
                }
            }
        }
    } else {
        let limit = value_t!(matches.value_of("limit"), i32).unwrap_or_else(|e| e.exit());
        client(multicast_addr, IPV4_MULTICAST_PORT, limit)
    }
}
