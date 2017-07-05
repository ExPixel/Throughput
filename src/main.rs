extern crate clap;

use clap::{App, Arg};
use std::io::{stdin, stdout, Write, Read};
use std::time::{Duration, Instant};
use std::net::{SocketAddr, TcpListener, IpAddr};

const BUFFER_SIZE: usize = 4096;
const ITERATIONS_PER_BUFFER: usize = 1;
const DEFAULT_address: &'static str = "127.0.0.1";

macro_rules! print_err {
    ($fmt:expr) => ({
        use std::io::{stderr, Write};
        if let Err(e) = write!(stderr(), $fmt) {
            panic!("Error while writing to stderr: {}", e);
        }
    });

    ($fmt:expr, $($arg:tt)*) => ({
        use std::io::{stderr, Write};
        if let Err(e) = write!(stderr(), $fmt, $($arg)*) {
            panic!("Error while writing to stderr: {}", e);
        }
    });
}

#[derive(Default)]
struct TransferInfo {
    /// The total number of bytes transferred.
    total_bytes_transferred: usize,

    /// The number of times the Bytes Per Second has been measured.
    total_measures: usize,

    /// Accumulation of all of the Bytes Per Second measures.
    total_bps: f64,

    /// The Bytes Per Second during the last measure.
    last_bps: f64,

    /// The number of bytes transferred during the last measure.
    last_bytes_transferred: usize,
}

fn main() {
    let matches = App::new("Throughput")
        .version("1.0")
        .author("Adolph C.")
        .about("Measures the throughput of stdin or a socket.")
        .arg(Arg::with_name("address")
            .short("i")
            .long("addr")
            .value_name("IP Address")
            .help("IP address to listen to. Defaults to 127.0.0.1. Must specify port.")
            .takes_value(true))
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .value_name("PORT_NUMBER")
            .help("Port to listen on. Must be specified if address is given.")
            .takes_value(true))
        .after_help("If a port/address is not specified, throughput will read from stdin.")
        .get_matches();

    let address_present = matches.is_present("address");
    let port_present = matches.is_present("port");
    if address_present || port_present {
        if !port_present {
            print_err!("A port must be speicified alongside a address.");
            std::process::exit(1);
        } else {
            let address = matches.value_of("address").unwrap_or(DEFAULT_address);
            let port = matches.value_of("port").expect("Expected port arg to have value.");

            if let Ok(parsed_port) = port.parse() {
                measure_tcp_stream(address, parsed_port);
            } else {
                print_err!("Port must be a valid number from 0 to 65535");
                std::process::exit(1);
            }
        }
    } else {
        measure_stdin();
    }
}

fn measure_tcp_stream(address: &str, port: u16) {
    let parsed_addr: IpAddr = match address.parse() {
        Ok(parsed) => parsed,
        Err(_) => {
            print_err!("Bad IP address {}", address);
            std::process::exit(1);
        }
    };

    let socket_addr = SocketAddr::new(parsed_addr, port);
    match TcpListener::bind(socket_addr) {
        Ok(listener) => {
            println!("Listening at {}", socket_addr);

            match listener.accept() {
                Ok((stream, incoming_addr)) => {
                    println!("Reading incoming data from {}", incoming_addr);
                    println!();
                    measure_reader(stream);
                },

                Err(err) => {
                    print_err!("There was an error accepting a connection.");
                    print_err!("ERROR: {}", err);
                    std::process::exit(1);
                }
            }
        },

        Err(err) => {
            print_err!("There was an error connecting to {}", socket_addr);
            print_err!("ERROR: {}", err);
            std::process::exit(1);
        }
    };
}

fn measure_stdin() {
    let input = stdin();
    measure_reader(input.lock());
}

fn measure_reader<R: Read>(mut reader: R) {
    let output = stdout();
    let mut locked_output = output.lock();
    
    let mut buffer = Vec::with_capacity(BUFFER_SIZE);
    buffer.resize(BUFFER_SIZE, 0);

    let mut last_measured = Instant::now();
    let mut transfer_info = TransferInfo::default();

    loop {
        let mut end_loop = false;
        for _ in 0..ITERATIONS_PER_BUFFER {
            match reader.read(&mut buffer) {
                Ok(bytes_read) => {
                    transfer_info.last_bytes_transferred += bytes_read;
                    transfer_info.total_bytes_transferred += bytes_read;
                    if bytes_read == 0 { end_loop = true; break; }
                }
                Err(err) => {
                    print_err!("Error while reading into buffer: {}", err);
                }
            }
        }

        let measure_end = Instant::now();
        let duration = measure_end.duration_since(last_measured);
        if duration.as_secs() > 0 || end_loop {
            transfer_info.last_bps = bytes_per_second(transfer_info.last_bytes_transferred, duration);
            transfer_info.total_measures += 1;
            transfer_info.total_bps += transfer_info.last_bps;
            print_info(&mut locked_output, &mut transfer_info);

            last_measured = measure_end;
            transfer_info.last_bps = 0.0;
            transfer_info.last_bytes_transferred = 0;
        }

        if end_loop { return; }
    }
}

fn print_info<W: Write>(output: &mut W, transfer_info: &mut TransferInfo) {
    if transfer_info.total_measures > 1 { term_move_up(output, 3); }

    let (mem_total_transfer, unit_total_transfer) = byte_to_mem_units(transfer_info.total_bytes_transferred as f64);
    print_fixed_width(output, "Data Transferred:", 24);
    write!(output, "{:.3} {} ({} cycles)", 
        mem_total_transfer, unit_total_transfer, transfer_info.total_measures);
    term_clear_line(output);

    let (mem_single, unit_single) = byte_to_mem_units(transfer_info.last_bps);
    print_fixed_width(output, "Transfer Speed:", 24);
    write!(output, "{:.3} {}/sec", mem_single, unit_single);
    term_clear_line(output);

    let avg_bps = transfer_info.total_bps / transfer_info.total_measures as f64;
    let (mem_avg, unit_avg) = byte_to_mem_units(avg_bps);
    print_fixed_width(output, "Average Transfer Speed:", 24);
    write!(output, "{:.3} {}/sec", mem_avg, unit_avg);
    term_clear_line(output);
}

fn print_fixed_width<W: Write>(output: &mut W, text: &str, columns: usize) {
    if let Err(err) = output.write(text.as_bytes()) {
        panic!("[print_fixed_width] Error while writing to stream: {}", err);
    }

    if text.len() < columns {
        let remaining = columns - text.len();

        let pad = [b' '];
        for _ in 0..remaining {
            if let Err(err) = output.write(&pad) {
                panic!("[print_fixed_width] Error while padding output: {}", err);
            }
        }
    }
}

/// Clears to the end of the current line.
#[inline]
fn term_clear_line<W: Write>(output: &mut W) {
    writeln!(output, "\x1b[K");
}

/// Moves the cursor up one line.
#[inline]
fn term_move_up<W: Write>(output: &mut W, lines: usize) {
    write!(output, "\x1b[{}A", lines);
}

fn byte_to_mem_units(bytes: f64) -> (f64, &'static str) {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    if bytes >= TB { (bytes / TB, "TB") }
    else if bytes >= GB { (bytes / GB, "GB") }
    else if bytes >= MB { (bytes / MB, "MB") }
    else if bytes >= KB { (bytes / KB, "KB") }
    else { (bytes, "Bytes") }
}

fn bytes_per_second(bytes_read: usize, duration: Duration) -> f64 {
    let duration_seconds = 
        duration.as_secs() as f64 + 
        duration.subsec_nanos() as f64 / 1000000000.0;
    return bytes_read as f64 / duration_seconds;
}