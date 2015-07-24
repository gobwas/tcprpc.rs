use ::rustc_serialize::json::{Json, ToJson, encode};
use ::rustc_serialize::{Encodable, Encoder};

#[derive(Debug)]
pub enum Response {
    Success(Success),
    Error(Error)
}

impl Encodable for Response {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        match *self {
            Response::Success(ref v) => v.encode(s),
            Response::Error(ref e) => e.encode(s),
        }
    }
}

#[derive(Debug)]
pub enum Id {
    Exists(String),
    Empty
}

impl Encodable for Id {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        self.to_json().encode(s)
    }
}

impl ToJson for Id {
    fn to_json(&self) -> Json {
        match *self {
            Id::Exists(ref v) => Json::String(v.clone()),
            Id::Empty => Json::Null,
        }
    }
}

impl Response {
    pub fn get_id(&self) -> Option<String> {
        match *self {
            Response::Success(ref success) => {
                match (success.id) {
                    Id::Exists(ref s) => {
                        Some(s.clone())
                    }
                    Id::Empty => {
                        None
                    }
                }
            }
            Response::Error(ref error) => {
                match (error.id) {
                    Id::Exists(ref s) => {
                        Some(s.clone())
                    }
                    Id::Empty => {
                        None
                    }
                }
            }
        }
    }
}

#[derive(Debug, RustcEncodable)]
pub struct Success {
    pub id: Id,
    pub result: Json
}

#[derive(Debug, RustcEncodable)]
pub struct ErrorDescription {
    pub code: i64,
    pub message: String
}

#[derive(Debug, RustcEncodable)]
pub struct Error {
    pub id: Id,
    pub error: ErrorDescription
}
