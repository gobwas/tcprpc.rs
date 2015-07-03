use ::rustc_serialize::json;
use ::rustc_serialize::json::{Json, Object};

#[derive(RustcEncodable)]
pub struct Request {
    pub id: String,
    pub topic: String,
    pub params: Vec<Json>
}

impl Request {
    pub fn new(id: String, topic: String, params: Vec<Json>) -> Request {
        Request { id: id, topic: topic, params: params }
    }
}
