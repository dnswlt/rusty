use clap::{value_t, App, Arg};
use regex::Regex;
use std::io;
use std::net::UdpSocket;
use std::num::ParseIntError;

fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

fn main() -> io::Result<()> {
    let matches = App::new("wake-on-lan")
        .version("0.1")
        .author("Dennis Walter <dennis.walter@gmail.com>")
        .about("Wake-on-lan by sending a magic packet.")
        .arg(
            Arg::with_name("mac-addr")
                .short("m")
                .long("mac-addr")
                .help("MAC address in ff:ff:ff:ff:ff:ff format")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("broadcast-addr")
                .short("b")
                .long("broadcast-addr")
                .help("Broadcast address (e.g. 192.168.0.255)")
                .default_value("255.255.255.255")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .help("UDP port to use")
                .default_value("9")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Print verbose output"),
        )
        .get_matches();
    let verbose = matches.is_present("verbose");
    let mac_addr = value_t!(matches.value_of("mac-addr"), String).unwrap_or_else(|e| e.exit());
    let broadcast_addr =
        value_t!(matches.value_of("broadcast-addr"), String).unwrap_or_else(|e| e.exit());
    let port = value_t!(matches.value_of("port"), u16).unwrap_or_else(|e| e.exit());
    let mac_re = Regex::new("^[0-9a-fA-F]{2}(:[0-9a-fA-F]{2}){5}$").expect("Broken regex");
    assert!(mac_re.is_match(&mac_addr));
    let mac_bytes = decode_hex(&str::replace(&mac_addr, ":", "")).expect("Invalid hex");
    let mut data = decode_hex("ffffffffffff").unwrap();
    for _ in 0..16 {
        data.extend(&mac_bytes);
    }
    if verbose {
        println!(
            "Sending wake-on-lan packet to {} on broacast address {}:{}",
            mac_addr, broadcast_addr, port
        );
    }
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_broadcast(true)?;
    sock.send_to(&data[..], format!("{}:{}", broadcast_addr, port))?;
    Ok(())
}
