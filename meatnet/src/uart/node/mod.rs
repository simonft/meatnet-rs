mod node_message;
pub use node_message::*;

pub mod request;
pub mod response;

use deku::prelude::*;

use request::Request;
use response::Response;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(ctx = "message_type_id: u8", id = "message_type_id")]
pub enum MessageType {
    #[deku(id = "0")]
    Request(Request),
    #[deku(id = "1")]
    Response(Response),
}
