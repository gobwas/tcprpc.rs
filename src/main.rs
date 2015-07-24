#[macro_use] extern crate log;
extern crate communicatio;
extern crate env_logger;
extern crate rustc_serialize;

use ::std::io::prelude::*;
use communicatio::rpc::server::{Server, HandlerResult};
use communicatio::rpc::response::ErrorDescription;
use ::rustc_serialize::json::{Json};

fn main() {
    env_logger::init().unwrap();

    debug!("Main starting...");

    let mut server = Server::new();
    server.handle("render", |params: Vec<Json>| {
        Some(Ok(Json::String("Hi".to_string())))
    });

    server.listen("127.0.0.1:3000");
}
