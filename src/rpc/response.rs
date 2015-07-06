use ::rustc_serialize::json::{Json};

#[derive(Debug)]
pub enum Response {
    Success(Success),
    Error(Error)
}

impl Response {
    pub fn get_id(&self) -> String {
        match *self {
            Response::Success(ref success) => {
                success.id.clone()
            }
            Response::Error(ref error) => {
                error.id.clone()
            }
        }
    }
}

#[derive(Debug)]
pub struct Success {
    pub id: String,
    pub result: Json
}

#[derive(Debug)]
pub struct ErrorDescription {
    pub code: i64,
    pub message: String
}

#[derive(Debug)]
pub struct Error {
    pub id: String,
    pub error: ErrorDescription
}
