use ::std::io::prelude::*;
use ::std::io;
use ::std::result::Result;
use ::std::net::{SocketAddr, TcpStream};
use ::rustc_serialize::json;
use ::rustc_serialize::json::{Json, Object};
use ::uuid::Uuid;
use ::std::thread;
use ::std::thread::{JoinHandle};
use ::std::sync::{Arc, Mutex, RwLock};
use ::std::collections::{HashMap};
use ::std::sync::mpsc::channel;
use ::std::sync::mpsc::{Sender, Receiver};
use ::std::io::BufReader;

use ::rpc::request::{Request};
use ::rpc::response::{Response, Id, Success, Error, ErrorDescription};
use ::rpc::response::Error as ResponseError;
use ::rpc::DELIMITER;

#[derive(Debug)]
pub enum ClientError {
    ParseError(json::ParserError),
    IoError(io::Error),
    IdMismatchError,
    UnknownError
}

pub type RequestResult = Result<Response, ClientError>;

pub struct Client {
    addr: SocketAddr,
    response_pool: Arc<Mutex<HashMap<String, Sender<Response>>>>,
    ready: bool,
    stream: Arc<RwLock<TcpStream>>
}

impl Client {
    // Constructor
    pub fn new(addr: SocketAddr) -> Client {
        let stream = match TcpStream::connect(addr) {
            Ok(stream) => {
                debug!("Connected to the host {}", addr);
                stream
            }
            Err(e) => {
                panic!("Could not connect to host {}", addr);
            }
        };

        Client {
            addr: addr,
            response_pool: Arc::new(Mutex::new(HashMap::new())),
            ready: false,
            stream: Arc::new(RwLock::new(stream))
        }
    }

    pub fn initialize(&mut self) {
        if self.ready == true {
            panic!("Already initialized");
        }

        let mut stream = self.stream.clone();
        let mut request = self.response_pool.clone();

        thread::spawn(move || {
            let mut stream = match stream.read() {
                Ok(s) => {
                    s.try_clone().unwrap()
                }
                _ => {
                    panic!("Could not lock stream");
                }
            };

            let mut reader = BufReader::new(stream);

            loop {
                let mut buffer: Vec<u8> = Vec::new();

                match reader.read_until(DELIMITER, &mut buffer) {
                    Ok(bytes_read) => {
                        if bytes_read == 0 {
                            panic!("Socket hunged up");
                        }

                        let obj = match Json::from_str(&*String::from_utf8(buffer).unwrap()) {
                            Ok(obj) => {
                                match obj {
                                    Json::Object(obj) => {
                                        debug!("Parsed response: {:?}", obj);
                                        obj
                                    }
                                    _ => {
                                        error!("Not an obj {:?}", obj);
                                        continue;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Could not parse json: {}", e);
                                continue;
                            }
                        };

                        let resp = match obj.get("error") {
                            Some(&Json::Object(ref err_def)) => {
                                let ( id, code, message ) = match ( obj.get("id"), err_def.get("code"), err_def.get("message") ) {
                                    ( Some(&Json::String(ref id)), Some(&Json::I64(code)), Some(&Json::String(ref message)) ) => {
                                        (id.clone(), code, message.clone())
                                    }
                                    _ => {
                                        error!("Unknown error response format: {:?}", obj);
                                        continue;
                                    }
                                };

                                Response::Error(ResponseError{
                                    id: Id::Exists(id),
                                    error: ErrorDescription {
                                        code: code,
                                        message: message
                                    }
                                })
                            }
                            _ => {
                                let (id, result) = match ( obj.get("id"), obj.get("result") ) {
                                    ( Some(&Json::String(ref id)), Some(result) ) => {
                                        ( id.clone(), result.clone() )
                                    }
                                    _ => {
                                        error!("Unknown response format: {:?}", obj);
                                        continue;
                                    }
                                };

                                Response::Success(Success{
                                    id: Id::Exists(id),
                                    result: result
                                })
                            }
                        };

                        info!("Got response {:?}", resp);
                        {

                            match (resp.get_id()) {
                                Some(ref id) => {
                                    info!("Locking request...");
                                    match request.lock().unwrap().remove(id) {
                                        Some(tx) => {
                                            tx.send(resp);
                                        }
                                        _ => {
                                            error!("No such channel");
                                            continue;
                                        }
                                    }
                                }
                                None => {
                                    //
                                }
                            };


                        }

                        info!("Unlocking request...");
                    }
                    Err(e) => {
                        error!("Read error :-(");
                    }
                }
            };
        });

        self.ready = true;
    }

    // Creates request
    pub fn request(&self, topic: &str, params: Vec<Json>) -> RequestResult {
        info!("Should request {:?}", topic);

        if self.ready == false {
            panic!("Could not send request to the closed client");
        }

        let topic = topic.to_string();
        let request_id = Uuid::new_v4().to_string();

        let (tx, rx) = channel();

        let mut pool = self.response_pool.clone();
        let mut stream = self.stream.clone();

        {
            pool.lock().unwrap().insert(request_id.clone(), tx);
            info!("Inserted in the response hash {:?}", request_id);

            info!("Locking pool...");
            {
                match stream.write() {
                    Ok(s) => {
                        let mut stream = s.try_clone().unwrap();
                        let req = Request::new(request_id.clone(), topic, params);

                        let _ = stream.write(json::encode(&req).unwrap().as_bytes());
                        let _ = stream.write(&[DELIMITER]);
                        info!("Sent requested to the host {:?}", req);
                    }
                    _ => {
                        panic!("Could not lock stream");
                    }
                };
            }
            info!("Unlocking pool...");
        }

        match rx.recv() {
            Ok(result) => {
                Ok(result)
            }
            Err(e) => {
                Err(ClientError::UnknownError)
            }
        }
    }
}
