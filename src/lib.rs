use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseMotion, MouseWheel},
};
use bevy_log::prelude::*;
use bevy_picking::PickSet;
use bevy_render::prelude::*;
use bevy_window::{PrimaryWindow, WindowEvent, prelude::*};

use gst_webrtc_encoder::GstWebRtcEncoder;

mod helper;
mod settings;

pub mod gst_webrtc_encoder;
#[cfg(feature = "ue_pixelstreaming")]
mod ue_pixelstreaming;

#[derive(Component)]
struct Encoder(GstWebRtcEncoder);

#[derive(Component)]
enum ControllerState {
    None,
    #[cfg(feature = "ue_pixelstreaming")]
    UeControllerState(UeControllerState),
}

pub use helper::*;
pub use settings::*;

#[cfg(feature = "ue_pixelstreaming")]
use ue_pixelstreaming::{
    controller::UeControllerState,
    message::UeMessage,
    utils::{PSKeyCode, PixelStreamingConversions},
};

pub struct StreamerPlugin;

impl Plugin for StreamerPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(bevy_capture::CapturePlugin);

        app.add_systems(
            PreUpdate,
            (
                process_encoder_events,
                start_capturing,
                handle_controller_messages.in_set(PickSet::Input),
            ),
        );
        app.add_systems(PostUpdate, handle_controllers);
    }
}

/// Process gstreamer encoder's events
fn process_encoder_events(encoders: Query<&Encoder>) {
    for encoder in encoders.iter() {
        encoder.0.process_events().expect("Error processing events");
    }
}

/// Starts all ready streamers
fn start_capturing(mut streamers: Query<(&mut bevy_capture::Capture, &Encoder)>) {
    for (mut capture, encoder) in streamers.iter_mut() {
        if !capture.is_capturing() {
            capture.start(encoder.0.clone());
        }
    }
}

/// This system process added and removed message handlers and update controller state
/// And it process messages from UE
fn handle_controllers(mut controllers: Query<&mut ControllerState>) {
    for mut controller in controllers.iter_mut() {
        let controller = controller.as_mut();
        match controller {
            ControllerState::None => {}
            #[cfg(feature = "ue_pixelstreaming")]
            ControllerState::UeControllerState(ue_controller_state) => {
                for (peer_id, handler) in ue_controller_state.add_remove_handlers.try_iter() {
                    // add / remove handlers
                    match handler {
                        Some(handler) => ue_controller_state.handlers.insert(peer_id, handler),
                        None => ue_controller_state.handlers.remove(&peer_id),
                    };
                }
            }
        }
    }
}

/// This system process controller's messages
fn handle_controller_messages(
    mut controllers: Query<(&Camera, &mut ControllerState)>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    #[cfg(feature = "ue_pixelstreaming")] ps_conversions: PixelStreamingConversions,
    mut mouse_motion_event: EventWriter<MouseMotion>,
    mut mouse_button_input_events: EventWriter<MouseButtonInput>,
    mut mouse_wheel_events: EventWriter<MouseWheel>,
    mut window_events: EventWriter<WindowEvent>,
    mut keyboard_input_events: EventWriter<KeyboardInput>,
) {
    let window = windows.get_single().unwrap().0;

    for (camera, mut controller) in controllers.iter_mut() {
        let controller = controller.as_mut();
        match controller {
            ControllerState::None => {}
            #[cfg(feature = "ue_pixelstreaming")]
            ControllerState::UeControllerState(ue_controller_state) => {
                for (_peer_id, handler) in ue_controller_state.handlers.iter() {
                    for ue_msg in handler.message_receiver.try_iter() {
                        match ue_msg {
                            UeMessage::MouseMove(mouse_move) => {
                                mouse_motion_event.send(MouseMotion {
                                    delta: ps_conversions.from_ps_delta(
                                        camera,
                                        mouse_move.delta_x,
                                        mouse_move.delta_y,
                                    ),
                                });
                                window_events.send(WindowEvent::CursorMoved(CursorMoved {
                                    window,
                                    position: ps_conversions.from_ps_position(
                                        camera,
                                        mouse_move.x,
                                        mouse_move.y,
                                    ),
                                    delta: Some(ps_conversions.from_ps_delta(
                                        camera,
                                        mouse_move.delta_x,
                                        mouse_move.delta_y,
                                    )),
                                }));
                            }
                            UeMessage::MouseDown(mouse_down) => {
                                mouse_button_input_events.send(MouseButtonInput {
                                    button: ps_conversions.ps_to_mouse_button(mouse_down.button),
                                    state: bevy_input::ButtonState::Pressed,
                                    window,
                                });
                            }
                            UeMessage::MouseUp(mouse_up) => {
                                mouse_button_input_events.send(MouseButtonInput {
                                    button: ps_conversions.ps_to_mouse_button(mouse_up.button),
                                    state: bevy_input::ButtonState::Released,
                                    window,
                                });
                            }
                            UeMessage::UiInteraction(ui_interaction) => {
                                info!("Received UiInteraction message: {:#?}", ui_interaction);
                            }
                            UeMessage::Command(command) => {
                                info!("Received Command message: {:#?}", command);
                            }
                            UeMessage::KeyDown(key_down) => {
                                info!("KeyDown {}", key_down.key_code);
                                keyboard_input_events.send(KeyboardInput {
                                    key_code: PSKeyCode(key_down.key_code).into(),
                                    logical_key: PSKeyCode(key_down.key_code).into(),
                                    state: bevy_input::ButtonState::Pressed,
                                    repeat: key_down.is_repeat == 1,
                                    window,
                                });
                            }
                            UeMessage::KeyUp(key_up) => {
                                info!("KeyUp {}", key_up.key_code);
                                keyboard_input_events.send(KeyboardInput {
                                    key_code: PSKeyCode(key_up.key_code).into(),
                                    logical_key: PSKeyCode(key_up.key_code).into(),
                                    state: bevy_input::ButtonState::Released,
                                    repeat: false,
                                    window,
                                });
                            }
                            UeMessage::KeyPress(key_press) => {
                                info!("KeyPress {}", key_press.char_code);
                            }
                            UeMessage::MouseEnter => {}
                            UeMessage::MouseLeave => {}
                            UeMessage::MouseWheel(mouse_wheel) => {
                                info!("MouseWheel {:#?}", mouse_wheel);
                                mouse_wheel_events.send(MouseWheel {
                                    unit: bevy_input::mouse::MouseScrollUnit::Pixel,
                                    x: 0_f32,
                                    y: mouse_wheel.delta as f32 / 10.0,
                                    window,
                                });
                            }
                            UeMessage::MouseDouble(_mouse_double) => {}
                        }
                    }
                }
            }
        }
    }
}
