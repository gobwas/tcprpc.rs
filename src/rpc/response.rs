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
    code: i8,
    message: String
}

pub struct Error {
    id: String,
    error: ErrorDescription
}
