pub mod node;
pub mod probe;

pub trait EncapsulatableMessage {
    type Encapsulation;
    fn encapsulate(self) -> Self::Encapsulation;
}
