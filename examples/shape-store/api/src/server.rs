use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    thread,
};

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use crate::http::{Request, Response, allowed_origin, read_request, write_response};

const LISTEN_BACKLOG: i32 = 1024;
const MIN_WORKER_THREADS: usize = 64;
const MAX_WORKER_THREADS: usize = 128;
const WORKER_THREAD_STACK_BYTES: usize = 256 * 1024;

pub(crate) fn run(route: fn(&Request) -> Response) -> std::io::Result<()> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);
    let listener = bind_listener(port)?;
    let worker_count = thread::available_parallelism().map_or(MIN_WORKER_THREADS, |count| {
        count.get().clamp(MIN_WORKER_THREADS, MAX_WORKER_THREADS)
    });
    let mut workers = Vec::with_capacity(worker_count);

    for _index in 0..worker_count {
        let worker_listener = listener.try_clone()?;
        let worker = thread::Builder::new()
            .stack_size(WORKER_THREAD_STACK_BYTES)
            .spawn(move || accept_loop(&worker_listener, route))?;
        workers.push(worker);
    }

    for worker in workers {
        match worker.join() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => return Err(error),
            Err(_panic) => return Err(std::io::Error::other("request worker thread failed")),
        }
    }

    Ok(())
}

fn bind_listener(port: u16) -> std::io::Result<TcpListener> {
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_reuse_address(true)?;
    socket.bind(&SockAddr::from(address))?;
    socket.listen(LISTEN_BACKLOG)?;
    Ok(socket.into())
}

fn accept_loop(listener: &TcpListener, route: fn(&Request) -> Response) -> std::io::Result<()> {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle(stream, route) {
                    eprintln!("request failed: {error}");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }

    Ok(())
}

fn handle(mut stream: TcpStream, route: fn(&Request) -> Response) -> std::io::Result<()> {
    let request = read_request(&mut stream)?;
    let cors_origin = allowed_origin(&request.origin);

    if request.method == "OPTIONS" {
        return write_response(
            &mut stream,
            &Response {
                status: "204 No Content",
                body: String::new(),
            },
            &cors_origin,
        );
    }

    let response = route(&request);
    write_response(&mut stream, &response, &cors_origin)
}
