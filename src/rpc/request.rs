use ::rustc_serialize::json;
use ::rustc_serialize::json::{Json, Object};

#[derive(RustcEncodable)]
pub struct Request {
    id: String,
    topic: String,
    params: Vec<Json>
}

impl Request {
    fn new(id: String, topic: String, params: Vec<Json>) -> Request {
        Request { id: id, topic: topic, params: params }
    }
}
