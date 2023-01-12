use clap::Parser;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::{
    io::{prelude::*, BufReader},
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
        return Ok(())
    }
    let mut stream = TcpStream::connect((args.host, args.port))?;
    let in_stream = stream.try_clone()?;
    // Send command
    serde_json::to_writer(
        &stream,
        &NetwCommand::MeasureThroughput(MeasureThroughputParams {
            bytes_download: args.bytes_download,
            bytes_upload: args.bytes_upload,
        }),
    )?;
    let zero: [u8; 1] = [0; 1];
    stream.write(&zero)?;
    stream.flush()?;
    let mut buf_reader = BufReader::new(in_stream);
    if bytes_download > 0 {
        // Download bytes
        let dl_started = Instant::now();
        consume_bytes(bytes_download, &mut buf_reader)?;
        let dl_elapsed = dl_started.elapsed().as_micros();
        let dl_rate = bytes_download as f64 / dl_elapsed as f64;
        println!("Download completed: {bytes_download} bytes in {dl_elapsed}us ({dl_rate:.3}MB/s)",);
    }
    if bytes_upload > 0 {
        // Upload bytes
        let up_started = Instant::now();
        send_bytes(args.bytes_upload, &mut stream)?;
        // To measure end-to-end throughput, wait for an ACK from the other side that all data has arrived.
        consume_bytes(4, &mut buf_reader)?;
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
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let in_stream = stream.try_clone()?;
    let mut buf_reader = BufReader::new(in_stream);
    buf_reader.read_until(0, &mut buf)?;
    buf.pop();
    match serde_json::from_str::<NetwCommand>(std::str::from_utf8(&buf).unwrap()) {
        Ok(NetwCommand::MeasureThroughput(params)) => {
            println!(
                "Received MeasureThroughput command with params {:?}",
                params
            );
            return measure_throughput(buf_reader, stream, params);
        }
        Err(e) => {
            println!("Couldn't process command: {:?}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ));
        }
    }
}

fn measure_throughput(
    mut buf_reader: BufReader<TcpStream>,
    mut out_stream: TcpStream,
    params: MeasureThroughputParams,
) -> std::io::Result<()> {
    // Send bytes for download
    if params.bytes_download > 0 {
        send_bytes(params.bytes_download, &mut out_stream)?;
    }
    if params.bytes_upload > 0 {
        consume_bytes(params.bytes_upload, &mut buf_reader)?;
        send_bytes(ACK_BYTES, &mut out_stream)?;
    }
    Ok(())
}

fn send_bytes(n_bytes: i64, out_stream: &mut TcpStream) -> std::io::Result<()> {
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
    Ok(())
}

fn consume_bytes(n_bytes: i64, buf_reader: &mut BufReader<TcpStream>) -> std::io::Result<()> {
    let mut buf = [0 as u8; 8192];
    let mut rem_bytes: i64 = n_bytes;
    while rem_bytes > 0 {
        let n_read = buf_reader.read(&mut buf)?;
        rem_bytes -= n_read as i64;
    }
    Ok(())
}
