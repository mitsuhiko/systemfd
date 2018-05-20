extern crate listenfd;
extern crate actix_web;

use listenfd::ListenFd;
use actix_web::{server, App, HttpRequest, Responder};

fn index(_req: HttpRequest) -> impl Responder {
    "Hello World!"
}

fn main() {
    let mut listenfd = ListenFd::from_env();
    let mut server = server::new(|| {
        App::new()
            .resource("/", |r| r.f(index))
    });

    server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        server.listen(l)
    } else {
        server.bind("127.0.0.1:3000").unwrap()
    };

    server.run();
}
