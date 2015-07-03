use ::rustc_serialize::json::{Json};

pub enum Response {
    Success(Success),
    Error(Error)
}

pub struct Success {
    id: String,
    result: Json
}

pub struct ErrorDescription {
    code: u64,
    message: String
}

pub struct Error {
    id: String,
    error: ErrorDescription
}
