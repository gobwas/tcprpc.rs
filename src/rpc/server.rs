use std::io::prelude::*;
use ::std::net::{SocketAddr, ToSocketAddrs, TcpStream, TcpListener};
use ::std::thread;
use ::std::io;
use ::std::boxed::{Box};
use ::std::clone::{Clone};
use ::std::fmt::{Debug, Display};
use ::std::collections::HashMap;
use ::std::sync::{Arc};
use ::std::sync::mpsc::{channel, Receiver, Sender};
use ::std::io::{BufReader};
use ::rustc_serialize::{Encodable};
use ::rustc_serialize::json;
use ::rustc_serialize::json::{Json, Object};

use ::rpc::request::{Request};
use ::rpc::response::{Response, Success, Id, ErrorDescription};
use ::rpc::response::Error as ReponseError;
use ::rpc::DELIMITER;

#[derive(Debug)]
pub enum ServerError {
    ParseError(json::ParserError),
    IoError(io::Error),
    IdMismatchError,
    UnknownError
}

pub type HandlerResult = Result<Json, ErrorDescription>;

pub struct Server<'a> {
    listener: Option<TcpListener>,
    handlers: HashMap<&'a str, Box<Fn(Vec<Json>) -> Option<HandlerResult> + 'a>>
}

pub struct Listening {
    pub addr: SocketAddr
}

fn send(stream: &mut TcpStream, resp: Response) {
    let _ = stream.write(json::encode(&resp).unwrap().as_bytes());
    let _ = stream.write(&[DELIMITER]);

    info!("Sent response {:?}", resp);
}

impl<'a> Server<'a> {
    pub fn new() -> Server<'a> {
        Server {
            listener: None,
            handlers: HashMap::new()
        }
    }

    pub fn handle<F: Fn(Vec<Json>) -> Option<HandlerResult> + 'a>(&mut self, topic: &'a str, handler: F) {
        self.handlers.insert(topic, Box::new(handler));
    }

    pub fn listen<A: ToSocketAddrs + Copy + Debug>(&mut self, addr: A) -> Result<Listening, ServerError> {
        let (tx, rx) = channel();

        let listener = match TcpListener::bind(addr) {
            Ok(l) => {
                info!("Listening {:?}..", addr);
                l
            }
            Err(e) => {
                error!("Could not listen {:?} - {}", addr, e);
                return Err(ServerError::IoError(e));
            }
        };

        self.listener = Some(listener.try_clone().unwrap());

        thread::spawn(move|| {
            // accept connections and process them, spawning a new thread for each one
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        tx.send(stream);
                    }
                    Err(e) => {
                        error!("Could not handle connection: {}", e);
                    }
                }
            }
        });

        let (transmitter, receiver) = channel();
        self.receive(rx, transmitter);

        for (stream, incoming) in receiver {
            let mut s = stream.try_clone().unwrap();

            match incoming {
                Ok(request) => {
                    match self.handlers.get(&*request.topic) {
                        Some(handler) => {
                            match handler(request.params) {
                                Some(resp) => {
                                    match resp {
                                        Ok(result) => {
                                            send(&mut s, Response::Success( Success {
                                                id: Id::Exists(request.id),
                                                result: result
                                            } ));
                                        }
                                        Err(desc) => {
                                            send(&mut s, Response::Error( ReponseError {
                                                id: Id::Exists(request.id),
                                                error: desc
                                            } ));
                                        }
                                    }
                                }
                                None => {
                                    // nothing has returned
                                    // todo run next
                                    // if noone returns some
                                    // return empty response
                                }
                            }
                        }
                        _ => {
                            debug!("No registerd handlers for '{}' topic", request.topic);
                            send(&mut s, Response::Error( ReponseError {
                                id: Id::Exists(request.id),
                                error: ErrorDescription {
                                    code: -32601,
                                    message: "Method not found".to_string()
                                }
                            } ));
                        }
                    }
                },
                Err(e) => {
                    send(&mut s, Response::Error( ReponseError {
                        id: Id::Empty,
                        error: ErrorDescription {
                            code: -32601,
                            message: e
                        }
                    } ));
                }
            };
        };

        Ok(Listening { addr: addr.to_socket_addrs().unwrap().next().unwrap() })
    }

    fn receive(&self, rx: Receiver<TcpStream>, tx: Sender<(TcpStream, Result<Request, String>)>) {
        thread::spawn(move|| {
            // receive connected client
            for stream in rx {
                let txx = tx.clone();

                // start new thread for comminication
                // with connected client
                thread::spawn(move|| {
                    loop {
                        let mut reader = BufReader::new(stream.try_clone().unwrap());
                        let mut buffer: Vec<u8> = Vec::new();
                        let s = stream.try_clone().unwrap();

                        let json = match reader.read_until(DELIMITER, &mut buffer) {
                            Ok(bytes_read) => {
                                if bytes_read == 0 {
                                    debug!("Socket hunged up");
                                    break;
                                }

                                String::from_utf8(buffer).unwrap()
                            }
                            _ => {
                                debug!("Error");
                                break;
                            }
                        };

                        let obj = match Json::from_str(&*json) {
                            Ok(Json::Object(obj)) => {
                                obj
                            }
                            _ => {
                                error!("Invalid json: {}", json);
                                txx.send((s, Err("Invalid".to_string())));
                                continue;
                            }
                        };

                        let (id, topic, params) = match ( obj.get("id"), obj.get("topic"), obj.get("params") ) {
                            ( Some(&Json::String(ref id)), Some(&Json::String(ref topic)), Some(&Json::Array(ref params)) ) => {
                                ( id.clone(), topic.clone(), params.clone() )
                            }
                            _ => {
                                info!("Unknown request format: {:?}", obj);
                                txx.send((s, Err("Malformed".to_string())));
                                continue;
                            }
                        };

                        let req = Request {
                            id:     id,
                            topic:  topic,
                            params: params
                        };

                        txx.send((s, Ok(req)));
                    }
                });
            }
        });
    }
}
