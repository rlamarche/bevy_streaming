use std::io::{Cursor, Read};

use anyhow::anyhow;
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Clone, Debug)]
pub enum UeMessage {
    UiInteraction(UiInteraction),
    Command(Command),
    KeyDown(KeyDown),
    KeyUp(KeyUp),
    KeyPress(KeyPress),
    MouseEnter,
    MouseLeave,
    MouseDown(MouseDown),
    MouseUp(MouseUp),
    MouseMove(MouseMove),
    MouseWheel(MouseWheel),
    MouseDouble(MouseDouble),
}

impl TryFrom<&[u8]> for UeMessage {
    type Error = anyhow::Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let Some(id) = value.get(0) else {
            return Err(anyhow!("Invalid buffer for decoding UeMessage"));
        };
        let Some(data) = value.get(1..) else {
            return Err(anyhow!("No data in buffer for decoding UeMessage"));
        };
        match id {
            50 => Ok(UeMessage::UiInteraction(UiInteraction::try_from(data)?)),
            51 => Ok(UeMessage::Command(Command::try_from(data)?)),
            60 => Ok(UeMessage::KeyDown(KeyDown::try_from(data)?)),
            61 => Ok(UeMessage::KeyUp(KeyUp::try_from(data)?)),
            62 => Ok(UeMessage::KeyPress(KeyPress::try_from(data)?)),
            70 => Ok(UeMessage::MouseEnter),
            71 => Ok(UeMessage::MouseLeave),
            72 => Ok(UeMessage::MouseDown(MouseDown::try_from(data)?)),
            73 => Ok(UeMessage::MouseUp(MouseUp::try_from(data)?)),
            74 => Ok(UeMessage::MouseMove(MouseMove::try_from(data)?)),
            75 => Ok(UeMessage::MouseWheel(MouseWheel::try_from(data)?)),
            76 => Ok(UeMessage::MouseDouble(MouseDouble::try_from(data)?)),
            _ => Err(anyhow!("Not supported message type {}", id)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UiInteraction {
    pub message: String,
}

impl TryFrom<&[u8]> for UiInteraction {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut rdr = Cursor::new(value);
        let mut message = String::new();
        rdr.read_to_string(&mut message)?;
        Ok(Self { message })
    }
}

#[derive(Clone, Debug)]
pub struct Command {
    pub command: String,
}

impl TryFrom<&[u8]> for Command {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut rdr = Cursor::new(value);
        let mut command = String::new();
        rdr.read_to_string(&mut command)?;
        Ok(Self { command })
    }
}

#[derive(Clone, Debug)]
pub struct KeyDown {
    pub key_code: u8,
    pub is_repeat: u8,
}

impl TryFrom<&[u8]> for KeyDown {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut rdr = Cursor::new(value);
        Ok(Self {
            key_code: rdr.read_u8()?,
            is_repeat: rdr.read_u8()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct KeyUp {
    pub key_code: u8,
}

impl TryFrom<&[u8]> for KeyUp {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut rdr = Cursor::new(value);
        Ok(Self {
            key_code: rdr.read_u8()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct KeyPress {
    pub char_code: u16,
}

impl TryFrom<&[u8]> for KeyPress {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut rdr = Cursor::new(value);
        Ok(Self {
            char_code: rdr.read_u16::<LittleEndian>()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct MouseMove {
    pub x: u16,
    pub y: u16,
    pub delta_x: i16,
    pub delta_y: i16,
}

impl TryFrom<&[u8]> for MouseMove {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut rdr = Cursor::new(value);
        Ok(Self {
            x: rdr.read_u16::<LittleEndian>()?,
            y: rdr.read_u16::<LittleEndian>()?,
            delta_x: rdr.read_i16::<LittleEndian>()?,
            delta_y: rdr.read_i16::<LittleEndian>()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct MouseDown {
    pub button: u8,
    pub x: u16,
    pub y: u16,
}

impl TryFrom<&[u8]> for MouseDown {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut rdr = Cursor::new(value);
        Ok(Self {
            button: rdr.read_u8()?,
            x: rdr.read_u16::<LittleEndian>()?,
            y: rdr.read_u16::<LittleEndian>()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct MouseUp {
    pub button: u8,
    pub x: u16,
    pub y: u16,
}

impl TryFrom<&[u8]> for MouseUp {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut rdr = Cursor::new(value);
        Ok(Self {
            button: rdr.read_u8()?,
            x: rdr.read_u16::<LittleEndian>()?,
            y: rdr.read_u16::<LittleEndian>()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct MouseWheel {
    pub delta: i16,
    pub x: u16,
    pub y: u16,
}

impl TryFrom<&[u8]> for MouseWheel {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut rdr = Cursor::new(value);
        Ok(Self {
            delta: rdr.read_i16::<LittleEndian>()?,
            x: rdr.read_u16::<LittleEndian>()?,
            y: rdr.read_u16::<LittleEndian>()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct MouseDouble {
    pub button: u8,
    pub x: u16,
    pub y: u16,
}

impl TryFrom<&[u8]> for MouseDouble {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut rdr = Cursor::new(value);
        Ok(Self {
            button: rdr.read_u8()?,
            x: rdr.read_u16::<LittleEndian>()?,
            y: rdr.read_u16::<LittleEndian>()?,
        })
    }
}
