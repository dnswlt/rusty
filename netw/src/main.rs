use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::{
    io::prelude::*,
    net::{TcpListener, TcpStream},
};

#[derive(Serialize, Deserialize, Debug)]
struct MeasureThroughputParams {
    bytes_download: u64,
    bytes_upload: u64,
}

#[derive(Serialize, Deserialize, Debug)]
enum NetwCommand {
    MeasureThroughput(MeasureThroughputParams),
}

#[derive(Parser, Debug)]
#[command(name = "netw")]
#[command(author = "Dennis Walter <dennis.walter@gmail.com>")]
#[command(version = "1.0")]
#[command(about = "Network performance testing", long_about = None)]
struct Args {
    #[arg(short = 'l', long, default_value_t = false)]
    server_mode: bool,
    #[arg(long, default_value_t = String::from("0.0.0.0"))]
    listen_addr: String,
    #[arg(short = 's', long, default_value_t = String::from("127.0.0.1"))]
    host: String,
    #[arg(short, long, default_value_t = 7878)]
    port: u16,
    #[arg(short = 'n', long, default_value_t = 1024 * 1024, value_parser = parse_num_with_units)]
    bytes_download: u64,
    #[arg(short = 'm', long, default_value_t = 0, value_parser = parse_num_with_units)]
    bytes_upload: u64,
    #[arg(long, default_value_t = 5000)]
    sock_timeout_millis: u64,
}

// Number of bytes to send to ACK reception of upload data.
const ACK_BYTES: u64 = 4;
// Buffer size for sending and receiving data.
const BUF_SIZE: usize = 8 * 1024;

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    if args.server_mode {
        return run_server(args);
    } else {
        return run_client(args);
    }
}

fn parse_num_with_units(s: &str) -> Result<u64, String> {
    let re = Regex::new(r"^(\d+)(([kmgtKMGT])(i)?[bB]?)?$").unwrap();
    if let Some(caps) = re.captures(s.trim()) {
        let b: u64 = caps.get(1).unwrap().as_str().parse().unwrap();
        let m = if caps.get(4).is_some() { 1024 } else { 1000 };
        if let Some(unit) = caps.get(3) {
            match unit.as_str().to_uppercase().as_str() {
                "K" => {
                    return Ok(b * m);
                }
                "M" => {
                    return Ok(b * m * m);
                }
                "G" => {
                    return Ok(b * m * m * m);
                }
                "T" => {
                    return Ok(b * m * m * m * m);
                }
                _ => {}
            }
        } else {
            return Ok(b);
        }
    }
    return Err(format!("Invalid number {}", s));
}

fn run_client(args: Args) -> std::io::Result<()> {
    let bytes_download = args.bytes_download;
    let bytes_upload = args.bytes_upload;
    if bytes_download == 0 && bytes_upload == 0 {
        println!("Nothing to do.");
        return Ok(());
    }
    let mut out_stream = TcpStream::connect((args.host, args.port))?;
    out_stream.set_write_timeout(Some(Duration::from_millis(args.sock_timeout_millis)))?;
    let in_stream = out_stream.try_clone()?;
    in_stream.set_read_timeout(Some(Duration::from_millis(args.sock_timeout_millis)))?;
    send_command(
        &NetwCommand::MeasureThroughput(MeasureThroughputParams {
            bytes_download: bytes_download,
            bytes_upload: bytes_upload,
        }),
        &out_stream,
    )?;
    if bytes_download > 0 {
        // Download bytes
        let dl_started = Instant::now();
        recv_bytes(bytes_download, &in_stream)?;
        let dl_elapsed = dl_started.elapsed().as_micros();
        let dl_rate = bytes_download as f64 / dl_elapsed as f64;
        println!(
            "Download completed: {bytes_download} bytes in {dl_elapsed}us ({dl_rate:.3} MB/s)",
        );
    }
    if bytes_upload > 0 {
        // Upload bytes
        let up_started = Instant::now();
        send_bytes(bytes_upload, &mut out_stream)?;
        // To measure end-to-end throughput, wait for an ACK from the other side that all data has arrived.
        recv_bytes(ACK_BYTES, &in_stream)?;
        let up_elapsed = up_started.elapsed().as_micros();
        let up_rate = bytes_upload as f64 / up_elapsed as f64;

        println!("Upload completed: {bytes_upload} bytes in {up_elapsed}us ({up_rate:.3} MB/s)",);
    }
    Ok(())
}

fn run_server(args: Args) -> std::io::Result<()> {
    let listener = TcpListener::bind((args.listen_addr, args.port))?;
    println!("Listening on {}", listener.local_addr().unwrap());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!(
                    "New incoming connection from {}",
                    stream.peer_addr().unwrap()
                );
                stream.set_read_timeout(Some(Duration::from_millis(args.sock_timeout_millis)))?;
                stream.set_write_timeout(Some(Duration::from_millis(args.sock_timeout_millis)))?;
                handle_connection(stream).unwrap_or_else(|e| {
                    println!("Unsuccessful connection: {}", e);
                });
            }
            Err(e) => {
                println!("Failed to accept new connection: {}", e);
                break;
            }
        }
    }
    Ok(())
}

fn handle_connection(stream: TcpStream) -> std::io::Result<()> {
    let out_stream = stream.try_clone()?;
    match recv_command(&stream)? {
        NetwCommand::MeasureThroughput(params) => {
            println!(
                "Received MeasureThroughput command with params {:?}",
                params
            );
            return measure_throughput(stream, out_stream, params);
        }
    }
}

fn send_command(cmd: &NetwCommand, mut out_stream: &TcpStream) -> std::io::Result<()> {
    let cmd_str = serde_json::to_string(&cmd)?;
    out_stream.write_all(&(cmd_str.as_bytes().len() as u32).to_be_bytes())?;
    out_stream.write_all(cmd_str.as_bytes())?;
    out_stream.flush()
}

fn recv_command(mut in_stream: &TcpStream) -> std::io::Result<NetwCommand> {
    let mut len_buf = [0 as u8; 4];
    in_stream.read_exact(&mut len_buf)?;
    let cmd_len = u32::from_be_bytes(len_buf);
    let mut buf: Vec<u8> = vec![0; cmd_len as usize];
    in_stream.read_exact(&mut buf)?;
    match serde_json::from_str::<NetwCommand>(std::str::from_utf8(&buf).unwrap()) {
        Ok(x) => Ok(x),
        Err(e) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        )),
    }
}

fn measure_throughput(
    in_stream: TcpStream,
    out_stream: TcpStream,
    params: MeasureThroughputParams,
) -> std::io::Result<()> {
    // Send bytes for download
    if params.bytes_download > 0 {
        send_bytes(params.bytes_download, &out_stream)?;
    }
    if params.bytes_upload > 0 {
        recv_bytes(params.bytes_upload, &in_stream)?;
        send_bytes(ACK_BYTES, &out_stream)?;
    }
    Ok(())
}

fn send_bytes(n_bytes: u64, mut out_stream: &TcpStream) -> std::io::Result<()> {
    let mut rem_bytes: u64 = n_bytes;
    let buf = vec![0x55; BUF_SIZE];
    while rem_bytes > 0 {
        let n_bytes = if rem_bytes < (BUF_SIZE as u64) {
            rem_bytes as usize
        } else {
            BUF_SIZE
        };
        let n_written = out_stream.write(&buf[0..n_bytes])?;
        rem_bytes -= n_written as u64;
    }
    out_stream.flush()
}

fn recv_bytes(n_bytes: u64, mut in_stream: &TcpStream) -> std::io::Result<()> {
    let mut buf = [0 as u8; BUF_SIZE];
    let mut rem_bytes: u64 = n_bytes;
    while rem_bytes > 0 {
        let n_read = in_stream.read(&mut buf)?;
        rem_bytes -= n_read as u64;
    }
    Ok(())
}
