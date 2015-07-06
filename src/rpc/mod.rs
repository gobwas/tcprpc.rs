mod request;
mod response;

use ::std::io::prelude::*;
use ::std::io;
use ::std::result::Result;
use ::std::net::{SocketAddr, TcpStream};
use ::rustc_serialize::json;
use ::rustc_serialize::json::{Json, Object};
use ::uuid::Uuid;
use ::std::thread;
use ::std::thread::{JoinHandle};
use ::std::sync::{Arc, Mutex};
use ::std::collections::{HashMap};

pub use self::request::{Request};
pub use self::response::{Response, Success, Error, ErrorDescription};
pub use self::response::Error as ResponseError;

pub const DELIMITER: &'static str = "----";
pub const SLEEP: u32 = 0;

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
    request_pool: Arc<Mutex<Vec<Request>>>,
    response_pool: Arc<Mutex<HashMap<String, Response>>>,
    ready: bool
}

impl Client {
    // Constructor
    pub fn new(addr: SocketAddr) -> Client {
        // let request: Vec<Request> = Vec::new();
        // let response: HashMap<String, Response> = HashMap::new();
        Client {
            addr: addr,
            request_pool: Arc::new(Mutex::new(Vec::new())),
            response_pool: Arc::new(Mutex::new(HashMap::new())),
            ready: false
        }
    }

    pub fn initialize(&mut self) {
        if self.ready == true {
            panic!("Already initialized");
        }

        let request = self.request_pool.clone();
        let response = self.response_pool.clone();

        let mut stream = match TcpStream::connect(self.addr) {
            Ok(stream) => {
                debug!("Connected to the host {}", self.addr);
                stream
            }
            Err(e) => {
                panic!("Could not connect to host {}", self.addr);
            }
        };

        let (mut streamInput, mut streamOutput) = match (stream.try_clone(), stream.try_clone()) {
            (Ok(input), Ok(output)) => {
                (input, output)
            }
            _ => {
                panic!("Could not prepare the stream");
            }
        };

        thread::spawn(move || {
            let mut tick = || {
                let mut pool = request.lock().unwrap();

                match pool.pop() {
                    Some(request) => {
                        let _ = streamInput.write(json::encode(&request).unwrap().as_bytes());
                        let _ = streamInput.write(DELIMITER.as_bytes());
                        info!("Sent requested to the host {:?}", request);
                    }
                    _ => {
                        // info!("No more requests to send");
                    }
                }
            };

            loop {
                // debug!("Writing tick");
                // thread::sleep_ms(SLEEP);
                tick();
            }
        });

        thread::spawn(move || {
            // buffer to store chunks from the server
            let mut buffer: Vec<String> = Vec::new();

            let mut tick = || {
                'reading: loop {
                    let mut buf = [0u8; 256];

                    match streamOutput.read(&mut buf) {
                        Ok(bytes_read) => {

                            if bytes_read == 0 {
                                panic!("Socket hanged up");
                            }

                            debug!("Read {} bytes", bytes_read);

                            let mut hash = response.lock().unwrap();

                            let hunk = match String::from_utf8(buf[..bytes_read].to_vec()) {
                                Ok(value) => {
                                    debug!("Received hunk {}", value);
                                    value
                                }
                                Err(e) => {
                                    error!("Invalid UTF-8 sequence: {}", e);
                                    buffer.clear();
                                    break 'reading;
                                    // return Err(ClientError::UnknownError);
                                }
                            };

                            let mut parts: Vec<&str> = hunk.split(DELIMITER).collect();

                            // if there is no delimeter in chunk
                            // then we need to store this chunk
                            // and wait for delimeter in upcoming chunks
                            if parts.len() == 1 {
                                buffer.push(hunk.clone());
                                continue 'reading;
                            }

                            'parsing: while parts.len() > 0 {
                                // push the last part before delimeter into the buffer
                                buffer.push(parts.remove(0).to_string());

                                // if there is no more delimeted chunks
                                // then the added above is non closed part of message
                                // so we need to wait for delimeter in upcoming chunks
                                if parts.len() == 0 {
                                    continue 'reading;
                                }

                                // join buffered parts
                                let collected = &buffer.connect("");

                                // clear it for upcoming parts
                                buffer.clear();

                                // join buffer and then parse
                                let obj: Object = match Json::from_str(collected) {
                                    Ok(obj) => {
                                        match obj {
                                            Json::Object(obj) => {
                                                debug!("Parsed response: {:?}", obj);
                                                obj
                                            }
                                            _ => {
                                                error!("Not an obj {:?}", obj);
                                                // return Err(ClientError::UnknownError);
                                                break 'parsing;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("Could not parse json: {}", e);
                                        // return Err(ClientError::ParseError(e));
                                        continue 'parsing;
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
                                                continue 'parsing;
                                                // return Err(ClientError::UnknownError)
                                            }
                                        };

                                        Response::Error(ResponseError{
                                            id: id,
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
                                                continue 'parsing;
                                                // return Err(ClientError::UnknownError);
                                            }
                                        };

                                        Response::Success(Success{
                                            id: id,
                                            result: result
                                        })
                                    }
                                };

                                info!("Got response {:?}", resp);
                                hash.insert(resp.get_id(), resp);
                            };
                        },
                        Err(e) => {
                            error!("Read error :-(");
                            continue 'reading;
                            // return Err(ClientError::IoError(e));
                        }
                    };
                };
            };

            loop {
                // debug!("Reading tick");
                // thread::sleep_ms(3000);
                tick();
            };
        });

        self.ready = true;
    }

    // Creates request
    pub fn request(&self, topic: &str, params: Vec<Json>) -> RequestResult {
        if self.ready == false {
            panic!("Could not send request to the closed client");
        }

        let request = self.request_pool.clone();
        let response = self.response_pool.clone();

        let topic = topic.to_string();

        let pending = thread::spawn(move || {
            let request_id = Uuid::new_v4().to_string();

            let mut write = || {
                let mut pool = match request.lock() {
                    Ok(p) => {
                        // debug!("Locked requests queue");
                        p
                    }
                    _ => {
                        panic!("Could not lock requests queue");
                    }
                };

                pool.push( Request::new(request_id.clone(), topic, params) );

                debug!("Pushed request to the queue ({})", pool.len());
            };

            let mut read = || {
                let mut pool = match response.lock() {
                    Ok(p) => {
                        // debug!("Locked responses queue");
                        p
                    }
                    _ => {
                        panic!("Could not lock responses queue");
                    }
                };

                pool.remove(&request_id)
            };

            write();

            'waiting: loop {
                // let resp: Option<Response> = hash.remove(&request_id);
                // debug!("Pending tick");
                // thread::sleep_ms(SLEEP);

                match read() {
                    Some(response) => {
                        return response;
                    }
                    _ => {
                        continue 'waiting;
                    }
                }
            }
        });

        debug!("Waiting for the thread result..");

        match pending.join() {
            Ok(result) => {
                Ok(result)
            }
            _ => {
                debug!("Opps");
                Err(ClientError::UnknownError)
            }
        }
    }
}
