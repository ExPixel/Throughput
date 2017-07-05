Throughput
====

USAGE:
---
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
    -l, --addr <IP Address>
            IP address to listen to. Defaults to 127.0.0.1. Must specify port.

    -b, --bufsize <BYTES>
            The size of the buffer used to read from the stream in bytes.
            Defaults to 4096.
    -i, --iterations <iterations>
            The number of times the buffer should be filled before a measure is
            taken. Defaults to 1.
    -p, --port <PORT_NUMBER>
            Port to listen on. Must be specified if address is given.


If a port/address is not specified, throughput will read from stdin.
```

EXAMPLES
---
```bash
cat /dev/random | throughput
```

```bash
throughput -p 8081

# And in another terminal:

yes | nc localhost 8081
```