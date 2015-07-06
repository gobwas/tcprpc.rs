mod request;
mod response;

use ::std::io::prelude::*;
use ::std::io;
use ::std::result::Result;
use ::std::net::{SocketAddr, TcpStream};
use ::rustc_serialize::json;
use ::rustc_serialize::json::{Json, Object};
use ::uuid::Uuid;

pub use self::request::{Request};
pub use self::response::{Response, Success, ErrorDescription};
pub use self::response::Error as ResponseError;

pub const DELIMITER: &'static str = "----";

pub enum ClientError {
    ParseError(json::ParserError),
    IoError(io::Error),
    IdMismatchError,
    UnknownError
}

pub type RequestResult<T> = Result<T, ClientError>;

pub struct Client {
    addr: SocketAddr
}

impl Client {
    // Constructor
    pub fn new(addr: SocketAddr) -> Client {
        Client {
            addr: addr
        }
    }

    // Creates request
    pub fn request(&self, topic: String, params: Vec<Json>) -> Result<Response, ClientError> {

        let mut stream = match TcpStream::connect(self.addr) {
            Ok(stream) => {
                println!("Connected to the host {}", self.addr);
                stream
            }
            Err(e) => {
                println!("Could not connect to host {}", self.addr);
                return Err(ClientError::IoError(e));
            }
        };

        let request_id = Uuid::new_v4().to_string();
        let request = Request::new( request_id.clone(), topic, params );

        let _ = stream.write(json::encode(&request).unwrap().as_bytes());
        let _ = stream.write(DELIMITER.as_bytes());

        // buffer to store chunks from the server
        let mut buffer: Vec<String> = Vec::new();

        'reading: loop {
            let mut buf = [0u8; 256];

            match stream.read(&mut buf) {
                Ok(bytes_read) => {
                    let hunk = match String::from_utf8(buf[..bytes_read].to_vec()) {
                        Ok(value) => {
                            value
                        }
                        Err(e) => {
                            println!("Invalid UTF-8 sequence: {}", e);
                            return Err(ClientError::UnknownError);
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

                    // push the last part before delimeter into the buffer
                    buffer.push(parts.remove(0).to_string());

                    // join buffer and then parse
                    let obj: Object = match Json::from_str(&buffer.connect("")) {
                        Ok(obj) => {
                            match obj {
                                Json::Object(obj) => {
                                    println!("parsed response: {:?}", obj);
                                    obj
                                }
                                _ => {
                                    println!("not obj {:?}", obj);
                                    return Err(ClientError::UnknownError);
                                }
                            }
                        }
                        Err(e) => {
                            println!("could not parse json: {}", e);
                            return Err(ClientError::ParseError(e));
                        }
                    };

                    let resp = match obj.get("error") {
                        Some(err_def) => {
                            let def: ( String, u64, String ) = match err_def {
                                &Json::Object(ref obj) => {
                                    match ( obj.get("id"), obj.get("code"), obj.get("message") ) {
                                        ( Some(&Json::String(ref id)), Some(&Json::U64(code)), Some(&Json::String(ref message)) ) => {
                                            (id.clone(), code, message.clone())
                                        }
                                        _ => {
                                            return Err(ClientError::UnknownError)
                                        }
                                    }
                                }
                                _ => {
                                    return Err(ClientError::UnknownError);
                                }
                            };

                            Response::Error(ResponseError{
                                id: def.0,
                                error: ErrorDescription {
                                    code: def.1,
                                    message: def.2
                                }
                            })
                        }
                        _ => {
                            let def: (String, Json) = match ( obj.get("id"), obj.get("result") ) {
                                ( Some(&Json::String(ref id)), Some(result) ) => {
                                    ( id.clone(), result.clone() )
                                }
                                _ => {
                                    return Err(ClientError::UnknownError);
                                }
                            };

                            Response::Success(Success{
                                id: def.0,
                                result: def.1
                            })
                        }
                    };

                    if resp.get_id() != request_id {
                        return Err(ClientError::IdMismatchError);
                    }

                    return Ok(resp);
                },
                Err(e) => {
                    println!("Read error :-(");
                    return Err(ClientError::IoError(e));
                }
            };
        };
    }
}
