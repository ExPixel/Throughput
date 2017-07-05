Throughput
====

USAGE:
```
Throughput 1.0
Adolph C.
Measures the throughput of stdin or a socket.

USAGE:
    throughput [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -i, --addr <IP Address>     IP address to listen to. Defaults to 127.0.0.1.
                                Must specify port.
    -p, --port <PORT_NUMBER>    Port to listen on. Must be specified if address
                                is given.

If a port/address is not specified, throughput will read from stdin.
```