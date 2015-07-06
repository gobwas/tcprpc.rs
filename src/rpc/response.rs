use ::rustc_serialize::json::{Json};

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
