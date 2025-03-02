use bevy_asset::prelude::*;
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_image::prelude::*;
use bevy_input::{
    keyboard::{Key, NativeKey, NativeKeyCode},
    prelude::*,
};
use bevy_log::prelude::*;
use bevy_math::prelude::*;
use bevy_picking::{pointer::Location, prelude::*};
use bevy_render::prelude::*;

pub const SCALE: f32 = 65536.0;

#[derive(SystemParam)]
pub struct PSConversions<'w> {
    images: Res<'w, Assets<Image>>,
}

impl<'w> PSConversions<'w> {
    pub fn from_ps_position<T>(&self, camera: &Camera, x: T, y: T) -> Vec2
    where
        T: Into<f32>,
    {
        let image = camera.target.as_image().unwrap();
        let image = self.images.get(image).unwrap();
        let w = image.size().x as f32;
        let h = image.size().y as f32;

        let x = Into::<f32>::into(x);
        let y = Into::<f32>::into(y);
        Vec2 {
            x: w * x as f32 / SCALE,
            y: h * y as f32 / SCALE,
        }
    }

    pub fn from_ps_delta<T>(&self, camera: &Camera, x: T, y: T) -> Vec2
    where
        T: Into<f32>,
    {
        self.from_ps_position(camera, x, y)
    }

    #[allow(dead_code)]
    pub fn ps_to_location(
        &self,
        camera: &Camera,
        window: Option<Entity>,
        x: u16,
        y: u16,
    ) -> Location {
        Location {
            target: camera.target.normalize(window).unwrap(),
            position: self.from_ps_position(camera, x, y),
        }
    }

    #[allow(dead_code)]
    pub fn ps_to_pointer_button(&self, button: u8) -> PointerButton {
        match button {
            0 => PointerButton::Primary,
            1 => PointerButton::Middle,
            2 => PointerButton::Secondary,
            _ => {
                warn!("Unhandeled button {}", button);
                PointerButton::Primary
            }
        }
    }

    pub fn ps_to_mouse_button(&self, button: u8) -> MouseButton {
        match button {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            _ => {
                warn!("Unhandeled button {}", button);
                MouseButton::Left
            }
        }
    }
}

pub struct PSKeyCode(pub u8);

impl Into<KeyCode> for PSKeyCode {
    fn into(self) -> KeyCode {
        match self.0 {
            8 => KeyCode::Backspace,
            9 => KeyCode::Tab,
            13 => KeyCode::Enter,
            16 => KeyCode::ShiftLeft,
            17 => KeyCode::ControlLeft,
            20 => KeyCode::CapsLock,
            32 => KeyCode::Space,
            // Arrows
            37 => KeyCode::ArrowLeft,
            38 => KeyCode::ArrowUp,
            39 => KeyCode::ArrowRight,
            40 => KeyCode::ArrowDown,
            54 => KeyCode::Minus,
            // A to Z
            65 => KeyCode::KeyA,
            66 => KeyCode::KeyB,
            67 => KeyCode::KeyC,
            68 => KeyCode::KeyD,
            69 => KeyCode::KeyE,
            70 => KeyCode::KeyF,
            71 => KeyCode::KeyG,
            72 => KeyCode::KeyH,
            73 => KeyCode::KeyI,
            74 => KeyCode::KeyJ,
            75 => KeyCode::KeyK,
            76 => KeyCode::KeyL,
            77 => KeyCode::KeyM,
            78 => KeyCode::KeyN,
            79 => KeyCode::KeyO,
            80 => KeyCode::KeyP,
            81 => KeyCode::KeyQ,
            82 => KeyCode::KeyR,
            83 => KeyCode::KeyS,
            84 => KeyCode::KeyT,
            85 => KeyCode::KeyU,
            86 => KeyCode::KeyV,
            87 => KeyCode::KeyW,
            88 => KeyCode::KeyX,
            89 => KeyCode::KeyY,
            90 => KeyCode::KeyZ,
            93 => KeyCode::ContextMenu,
            106 => KeyCode::NumpadMultiply,
            107 => KeyCode::NumpadAdd,
            109 => KeyCode::NumpadSubtract,
            110 => KeyCode::NumpadComma,
            111 => KeyCode::NumpadDivide,
            // F1..F12
            112 => KeyCode::F1,
            113 => KeyCode::F2,
            114 => KeyCode::F3,
            115 => KeyCode::F4,
            116 => KeyCode::F5,
            117 => KeyCode::F6,
            118 => KeyCode::F7,
            119 => KeyCode::F8,
            120 => KeyCode::F9,
            121 => KeyCode::F10,
            122 => KeyCode::F11,
            123 => KeyCode::F12,
            188 => KeyCode::Comma,
            190 => KeyCode::Semicolon,
            225 => KeyCode::AltRight,
            253 => KeyCode::ShiftRight,
            254 => KeyCode::ControlRight,
            _ => {
                warn!("Unimplemented keycode {}", self.0);
                KeyCode::Unidentified(NativeKeyCode::Unidentified)
            }
        }
    }
}

impl Into<Key> for PSKeyCode {
    fn into(self) -> Key {
        match self.0 {
            8 => Key::Backspace,
            9 => Key::Tab,
            13 => Key::Enter,
            16 => Key::Shift,
            17 => Key::Control,
            20 => Key::CapsLock,
            32 => Key::Space,
            // Arrows
            37 => Key::ArrowLeft,
            38 => Key::ArrowUp,
            39 => Key::ArrowRight,
            40 => Key::ArrowDown,
            54 => Key::Character("-".into()),
            // A to Z
            65 => Key::Character("a".into()),
            66 => Key::Character("b".into()),
            67 => Key::Character("c".into()),
            68 => Key::Character("d".into()),
            69 => Key::Character("e".into()),
            70 => Key::Character("f".into()),
            71 => Key::Character("g".into()),
            72 => Key::Character("h".into()),
            73 => Key::Character("i".into()),
            74 => Key::Character("j".into()),
            75 => Key::Character("k".into()),
            76 => Key::Character("l".into()),
            77 => Key::Character("m".into()),
            78 => Key::Character("n".into()),
            79 => Key::Character("o".into()),
            80 => Key::Character("p".into()),
            81 => Key::Character("q".into()),
            82 => Key::Character("r".into()),
            83 => Key::Character("s".into()),
            84 => Key::Character("t".into()),
            85 => Key::Character("u".into()),
            86 => Key::Character("v".into()),
            87 => Key::Character("w".into()),
            88 => Key::Character("x".into()),
            89 => Key::Character("y".into()),
            90 => Key::Character("z".into()),
            93 => Key::ContextMenu,
            // F1..F12
            106 => Key::Character("*".into()),
            107 => Key::Character("+".into()),
            109 => Key::Character("-".into()),
            110 => Key::Character(".".into()),
            111 => Key::Character("/".into()),
            112 => Key::F1,
            113 => Key::F2,
            114 => Key::F3,
            115 => Key::F4,
            116 => Key::F5,
            117 => Key::F6,
            118 => Key::F7,
            119 => Key::F8,
            120 => Key::F9,
            121 => Key::F10,
            122 => Key::F11,
            123 => Key::F12,
            188 => Key::Character(",".into()),
            190 => Key::Character(";".into()),
            225 => Key::AltGraph,
            253 => Key::Shift,
            254 => Key::Control,
            _ => {
                warn!("Unimplemented keycode {}", self.0);
                Key::Unidentified(NativeKey::Unidentified)
            }
        }
    }
}
