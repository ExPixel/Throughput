extern crate clap;

use clap::{App, Arg};
use std::io::{stderr, stdin, stdout, Write, Read};
use std::time::{Duration, Instant};
use std::net::{SocketAddr, TcpListener, IpAddr};

const DEFAULT_BUFFER_SIZE: usize = 4096;
const DEFAULT_ITERATION_COUNT: usize = 1;
const DEFAULT_ADDRESS: &'static str = "127.0.0.1";

macro_rules! print_err_into {
    ($err_write: expr, $fmt:expr) => ({
        use std::io::Write;
        if let Err(e) = writeln!($err_write, $fmt) {
            panic!("Error while writing to stderr: {}", e);
        }
    });

    ($err_write: expr, $fmt:expr, $($arg:tt)*) => ({
        use std::io::Write;
        if let Err(e) = writeln!($err_write, $fmt, $($arg)*) {
            panic!("Error while writing to stderr: {}", e);
        }
    });
}


macro_rules! print_err {
    ($fmt:expr) => ({
        use std::io::{stderr, Write};
        if let Err(e) = writeln!(stderr(), $fmt) {
            panic!("Error while writing to stderr: {}", e);
        }
    });

    ($fmt:expr, $($arg:tt)*) => ({
        use std::io::{stderr, Write};
        if let Err(e) = writeln!(stderr(), $fmt, $($arg)*) {
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

#[inline]
fn exit_err() -> ! {
    std::process::exit(1);
}

fn main() {
    let matches = App::new("Throughput")
        .version("1.1")
        .author("Adolph C.")
        .about("Measures the throughput of stdin or a socket.")
        .arg(Arg::with_name("address")
            .short("l")
            .long("addr")
            .value_name("IP Address")
            .help("IP address to listen to. Defaults to 127.0.0.1. Must specify port.")
            .takes_value(true))
        .arg(Arg::with_name("buffer_size")
            .short("b")
            .long("bufsize")
            .value_name("BYTES")
            .help("The size of the buffer used to read from the stream in bytes. Defaults to 4096.")
            .takes_value(true))
        .arg(Arg::with_name("iterations")
            .short("i")
            .long("iterations")
            .help("The number of times the buffer should be filled before a measure is taken. Defaults to 1.")
            .takes_value(true))
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .value_name("PORT_NUMBER")
            .help("Port to listen on. Must be specified if address is given.")
            .takes_value(true))
        .arg(Arg::with_name("pass")
            .long("pass")
            .help("If present, throughput will print to stderr and pass input to stdout.")
            .takes_value(false))
        .after_help("If a port/address is not specified, throughput will read from stdin.")
        .get_matches();

    let passthrough = matches.is_present("pass");
    let buffer_size: usize;
    let iterations: usize;

    if let Some(buf_size_str) = matches.value_of("buffer_size") {
        if let Ok(bsize) = buf_size_str.parse() {
            buffer_size = bsize;
        } else {
            print_err!("Buffer size must be a valid number.");
            exit_err();
        }
    } else {
        buffer_size = DEFAULT_BUFFER_SIZE;
    }


    if let Some(iterations_str) = matches.value_of("iterations") {
        if let Ok(it) = iterations_str.parse() {
            iterations = it;
        } else {
            print_err!("Iterations must be a valid number.");
            exit_err();
        }
    } else {
        iterations = DEFAULT_ITERATION_COUNT;
    }

    let address_present = matches.is_present("address");
    let port_present = matches.is_present("port");
    if address_present || port_present {
        if !port_present {
            print_err!("A port must be speicified alongside a address.");
            exit_err();
        } else {
            let address = matches.value_of("address").unwrap_or(DEFAULT_ADDRESS);
            let port = matches.value_of("port").expect("Expected port arg to have value.");

            if let Ok(parsed_port) = port.parse() {
                measure_tcp_stream(address, parsed_port, buffer_size, iterations, passthrough);
            } else {
                print_err!("Port must be a valid number from 0 to 65535");
                exit_err();
            }
        }
    } else {
        measure_stdin(buffer_size, iterations, passthrough);
    }
}

fn measure_tcp_stream(address: &str, port: u16, buffer_size: usize, iterations: usize, passthrough: bool) {
    let parsed_addr: IpAddr = match address.parse() {
        Ok(parsed) => parsed,
        Err(_) => {
            print_err!("Bad IP address {}", address);
            exit_err();
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
                    measure_reader(stream, buffer_size, iterations, passthrough);
                },

                Err(err) => {
                    print_err!("There was an error accepting a connection.");
                    print_err!("ERROR: {}", err);
                    exit_err();
                }
            }
        },

        Err(err) => {
            print_err!("There was an error connecting to {}", socket_addr);
            print_err!("ERROR: {}", err);
            exit_err();
        }
    };
}

fn measure_stdin(buffer_size: usize, iterations: usize, passthrough: bool) {
    let input = stdin();
    measure_reader(input.lock(), buffer_size, iterations, passthrough);
}

fn measure_reader<R: Read>(mut reader: R, buffer_size: usize, iterations: usize, passthrough: bool) {
    let output = stdout();
    let mut locked_output = output.lock();

    let err_out = stderr();
    let mut locked_error = err_out.lock();
    
    let mut buffer = Vec::with_capacity(buffer_size);
    buffer.resize(buffer_size, 0);

    let mut last_measured = Instant::now();
    let mut transfer_info = TransferInfo::default();

    loop {
        let mut end_loop = false;
        for _ in 0..iterations {
            match reader.read(&mut buffer) {
                Ok(bytes_read) => {
                    transfer_info.last_bytes_transferred += bytes_read;
                    transfer_info.total_bytes_transferred += bytes_read;
                    if bytes_read == 0 {
                        end_loop = true;
                        break;
                    } else if passthrough {
                        if let Err(err) = locked_output.write_all(&buffer[0..bytes_read]) {
                            print_err_into!(locked_error, "Error while writing buffer into stdout: {}", err);
                            exit_err();
                        }
                    }
                }

                Err(err) => {
                    print_err_into!(locked_error, "Error while reading into buffer: {}", err);
                }
            }
        }

        let measure_end = Instant::now();
        let duration = measure_end.duration_since(last_measured);
        if duration.as_secs() > 0 || end_loop {
            transfer_info.last_bps = bytes_per_second(transfer_info.last_bytes_transferred, duration);
            transfer_info.total_measures += 1;
            transfer_info.total_bps += transfer_info.last_bps;

            let _print_result = if passthrough {
                print_info(&mut locked_error, &mut transfer_info)
            } else {
                print_info(&mut locked_output, &mut transfer_info)
            };

            match _print_result {
                Ok(_) => {},
                Err(err) => {
                    print_err_into!(locked_error, "Error while printing output: {}", err);
                    exit_err();
                }
            }

            last_measured = measure_end;
            transfer_info.last_bps = 0.0;
            transfer_info.last_bytes_transferred = 0;
        }

        if end_loop { return; }
    }
}

fn print_info<W: Write>(output: &mut W, transfer_info: &mut TransferInfo) -> Result<(), std::io::Error> {
    if transfer_info.total_measures > 1 { term_move_up(output, 3)?; }

    let (mem_total_transfer, unit_total_transfer) = byte_to_mem_units(transfer_info.total_bytes_transferred as f64);
    print_fixed_width(output, "Data Transferred:", 24);
    write!(output, "{:.3} {} ({} cycles)", 
        mem_total_transfer, unit_total_transfer, transfer_info.total_measures)?;
    term_clear_line(output)?;

    let (mem_single, unit_single) = byte_to_mem_units(transfer_info.last_bps);
    print_fixed_width(output, "Transfer Speed:", 24);
    write!(output, "{:.3} {}/sec", mem_single, unit_single)?;
    term_clear_line(output)?;

    let avg_bps = transfer_info.total_bps / transfer_info.total_measures as f64;
    let (mem_avg, unit_avg) = byte_to_mem_units(avg_bps);
    print_fixed_width(output, "Average Transfer Speed:", 24);
    write!(output, "{:.3} {}/sec", mem_avg, unit_avg)?;
    term_clear_line(output)?;

    Ok(())
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
fn term_clear_line<W: Write>(output: &mut W) -> Result<(), std::io::Error> {
    writeln!(output, "\x1b[K")?;
    Ok(())
}

/// Moves the cursor up one line.
#[inline]
fn term_move_up<W: Write>(output: &mut W, lines: usize) -> Result<(), std::io::Error> {
    write!(output, "\x1b[{}A", lines)?;
    Ok(())
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