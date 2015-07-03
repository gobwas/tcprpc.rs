mod request;
mod response;

use ::std::io::prelude::*;
use ::std::io;
use ::std::result::Result;
use ::std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
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
    Unknown
}

pub type RequestResult<T> = Result<T, ClientError>;

pub struct Client {
    host: Ipv4Addr,
    port: u16
}

impl Client {
    // Constructor
    fn new(host: Ipv4Addr, port: u16) -> Client {
        Client {
            host: host,
            port: port
        }
    }

    // Creates request
    fn request(&self, topic: String, params: Vec<Json>) -> Result<Response, ClientError> {

        let mut stream = match TcpStream::connect(SocketAddrV4::new( Ipv4Addr::new(127, 0, 0, 1), 3000u16 )) {
            Ok(stream) => {
                println!("Connected to the host {}:{}", self.host, self.port);
                stream
            }
            Err(e) => {
                println!("Could not connect to host {}:{}", self.host, self.port);
                return Err(ClientError::IoError(e));
            }
        };

        let request = Request::new( Uuid::new_v4().to_string(), topic, params );

        let _ = stream.write(json::encode(&request).unwrap().as_bytes());
        let _ = stream.write(DELIMITER.as_bytes());

        // buffer to store chunks from the server
        let mut buffer: Vec<String> = Vec::new();

        'reading: loop {
            let mut buf = [0u8; 256];

            match stream.read(&mut buf) {
                Ok(bytes_read) => {
                    match String::from_utf8(buf[..bytes_read].to_vec()) {
                        Ok(value) => {
                            let mut parts: Vec<&str> = value.split(DELIMITER).collect();;

                            // if there is no delimeter in chunk
                            // then we need to store this chunk
                            // and wait for delimeter in upcoming chunks
                            if parts.len() == 1 {
                                buffer.push(value.clone());
                                continue 'reading;
                            }

                            // push the last part before delimeter into the buffer
                            buffer.push(parts.remove(0).to_string());

                            // compose buffer and parse it
                            let obj: Object = match Json::from_str(&buffer.connect("")) {
                                Ok(obj) => {
                                    match obj {
                                        Json::Object(obj) => {
                                            println!("parsed response: {:?}", obj);
                                            obj
                                        }
                                        _ => {
                                            println!("not obj {:?}", obj);
                                            return Err(ClientError::Unknown);
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("could not parse json: {}", e);
                                    return Err(ClientError::ParseError(e));
                                }
                            };


                            let id = obj.get("id").unwrap().to_string();

                            let resp = match obj.get("error") {
                                Some(err_def) => {
                                    let def: (u64, String) = match err_def {
                                        &Json::Object(ref obj) => {
                                            match ( obj.get("code"), obj.get("message") ) {
                                                ( Some(&Json::U64(c)), Some(&Json::String(ref m)) ) => {
                                                    (c, m.clone())
                                                }
                                                _ => {
                                                    return Err(ClientError::Unknown)
                                                }
                                            }
                                        }
                                        _ => {
                                            return Err(ClientError::Unknown);
                                        }
                                    };

                                    Response::Error(ResponseError{
                                        id: id,
                                        error: ErrorDescription {
                                            code: def.0,
                                            message: def.1
                                        }
                                    })
                                }
                                _ => {
                                    let def: (String, Json) = match ( obj.get("id"), obj.get("result") ) {
                                        ( Some(&Json::String(ref id)), Some(result) ) => {
                                            ( id.clone(), result.clone() )
                                        }
                                        _ => {
                                            return Err(ClientError::Unknown);
                                        }
                                    };

                                    Response::Success(Success{
                                        id: def.0,
                                        result: def.1
                                    })
                                }
                            };

                            return Ok(resp);

                            break 'reading;
                        },
                        Err(e) => {
                            println!("Invalid UTF-8 sequence: {}", e);
                            return Err(ClientError::Unknown);
                        }
                    }

                },
                Err(e) => {
                    println!("Read error :-(");
                    return Err(ClientError::Unknown);
                }
            };
        };
    }
}
