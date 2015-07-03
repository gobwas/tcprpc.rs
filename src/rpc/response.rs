use ::rustc_serialize::json::{Json};

pub enum Response {
    Success(Success),
    Error(Error)
}

pub struct Success {
    pub id: String,
    pub result: Json
}

pub struct ErrorDescription {
    pub code: u64,
    pub message: String
}

pub struct Error {
    pub id: String,
    pub error: ErrorDescription
}
