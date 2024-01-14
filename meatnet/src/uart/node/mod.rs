pub mod request;
pub mod response;

use deku::prelude::*;

use request::Request;
use response::Response;

pub enum MessageType {
    Request(Request),
    Response(Response),
}

pub fn try_request_or_response_from(input: &[u8]) -> Result<MessageType, DekuError> {
    match Request::try_from(input) {
        Ok(message) => Ok(MessageType::Request(message)),
        Err(_) => match Response::try_from(input) {
            Ok(message) => Ok(MessageType::Response(message)),
            Err(e) => Err(e),
        },
    }
}
