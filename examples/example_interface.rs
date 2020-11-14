use nanoserde::{DeJson, DeJsonErr, DeJsonState, SerJson, SerJsonState};
use std::str::Chars;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ConnectionId(pub u32);

impl DeJson for ConnectionId {
    fn de_json(s: &mut DeJsonState, i: &mut Chars) -> Result<Self, DeJsonErr> {
        Ok(ConnectionId(u32::de_json(s, i)?))
    }
}

impl SerJson for ConnectionId {
    fn ser_json(&self, d: usize, s: &mut SerJsonState) {
        self.0.ser_json(d, s);
    }
}

#[derive(Clone, Debug, DeJson, SerJson)]
pub struct MousePos {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, DeJson, SerJson)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Clone, Debug, DeJson, SerJson)]
pub struct ClientState {
    pub id: ConnectionId,
    pub color: Color,
    pub pos: MousePos,
}
