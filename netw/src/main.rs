use clap::Parser;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::{
    io::{prelude::*},
    net::{TcpListener, TcpStream},
};

#[derive(Serialize, Deserialize, Debug)]
struct MeasureThroughputParams {
    bytes_download: i64,
    bytes_upload: i64,
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
    #[arg(short = 's', long, default_value_t = String::from("127.0.0.1"))]
    host: String,
    #[arg(short, long, default_value_t = 7878)]
    port: u16,
    #[arg(short = 'n', long, default_value_t = 1000000)]
    bytes_download: i64,
    #[arg(short = 'm', long, default_value_t = 0)]
    bytes_upload: i64,
}

// Number of bytes to send to ACK reception of up/download data.
const ACK_BYTES: i64 = 4;

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    if args.server_mode {
        return run_server();
    } else {
        return run_client(args);
    }
}

fn run_client(args: Args) -> std::io::Result<()> {
    let bytes_download = args.bytes_download;
    let bytes_upload = args.bytes_upload;
    if bytes_download <= 0 && bytes_upload <= 0 {
        println!("Nothing to do.");
        return Ok(());
    }
    let mut out_stream = TcpStream::connect((args.host, args.port))?;
    let in_stream = out_stream.try_clone()?;
    send_command(
        &NetwCommand::MeasureThroughput(MeasureThroughputParams {
            bytes_download: args.bytes_download,
            bytes_upload: args.bytes_upload,
        }),
        &out_stream,
    )?;
    if bytes_download > 0 {
        // Download bytes
        let dl_started = Instant::now();
        consume_bytes(bytes_download, &in_stream)?;
        let dl_elapsed = dl_started.elapsed().as_micros();
        let dl_rate = bytes_download as f64 / dl_elapsed as f64;
        println!("Download completed: {bytes_download} bytes in {dl_elapsed}us ({dl_rate:.3}MB/s)",);
    }
    if bytes_upload > 0 {
        // Upload bytes
        let up_started = Instant::now();
        send_bytes(args.bytes_upload, &mut out_stream)?;
        // To measure end-to-end throughput, wait for an ACK from the other side that all data has arrived.
        consume_bytes(ACK_BYTES, &in_stream)?;
        let up_elapsed = up_started.elapsed().as_micros();
        let up_rate = bytes_upload as f64 / up_elapsed as f64;

        println!("Upload completed: {bytes_upload} bytes in {up_elapsed}us ({up_rate:.3}MB/s)",);
    }
    Ok(())
}

fn run_server() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:7878")?;
    println!("Listening on {}", listener.local_addr().unwrap());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!(
                    "New incoming connection from {}",
                    stream.peer_addr().unwrap()
                );
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
    mut in_stream: TcpStream,
    mut out_stream: TcpStream,
    params: MeasureThroughputParams,
) -> std::io::Result<()> {
    // Send bytes for download
    if params.bytes_download > 0 {
        send_bytes(params.bytes_download, &mut out_stream)?;
    }
    if params.bytes_upload > 0 {
        consume_bytes(params.bytes_upload, &mut in_stream)?;
        send_bytes(ACK_BYTES, &mut out_stream)?;
    }
    Ok(())
}

fn send_bytes(n_bytes: i64, mut out_stream: &TcpStream) -> std::io::Result<()> {
    let mut rem_bytes: i64 = n_bytes;
    const BUF_SIZE: usize = 8192;
    let buf = vec![42; BUF_SIZE];
    while rem_bytes > 0 {
        let n_bytes = if rem_bytes < (BUF_SIZE as i64) {
            rem_bytes as usize
        } else {
            BUF_SIZE
        };
        let n_written = out_stream.write(&buf[0..n_bytes])?;
        rem_bytes -= n_written as i64;
    }
    out_stream.flush()
}

fn consume_bytes(n_bytes: i64, mut in_stream: &TcpStream) -> std::io::Result<()> {
    let mut buf = [0 as u8; 8192];
    let mut rem_bytes: i64 = n_bytes;
    while rem_bytes > 0 {
        let n_read = in_stream.read(&mut buf)?;
        rem_bytes -= n_read as i64;
    }
    Ok(())
}
