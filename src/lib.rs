use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseMotion, MouseWheel},
};
use bevy_picking::PickSet;
use bevy_render::{Render, RenderApp, RenderSet, prelude::*, render_graph::RenderGraph};
#[cfg(feature = "pixelstreaming")]
use bevy_window::{PrimaryWindow, WindowEvent, prelude::*};

use capture::{
    capture_extract,
    driver::{CaptureDriver, CaptureLabel},
};

mod capture;
mod helper;
mod settings;

pub mod gst_webrtc_encoder;
#[cfg(feature = "pixelstreaming")]
mod pixelstreaming;
pub mod encoder;
#[cfg(feature = "livekit")]
pub mod livekit;

#[derive(Component)]
enum ControllerState {
    None,
    #[cfg(feature = "pixelstreaming")]
    PSControllerState(PSControllerState),
}
pub use helper::*;
pub use settings::*;

#[cfg(feature = "pixelstreaming")]
use pixelstreaming::{
    controller::PSControllerState,
    message::PSMessage,
    utils::{PSConversions, PSKeyCode},
};

use crate::capture::{
    ReleaseBufferSignal, WorkerSendBuffer,
    driver::{receive_image_from_buffer, release_mapped_buffers},
    spawn_worker,
};

pub struct StreamerPlugin;

impl Plugin for StreamerPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let render_app = app.sub_app_mut(RenderApp);

        let (tx_job, rx_release) = spawn_worker();

        render_app.insert_resource(WorkerSendBuffer { tx: tx_job });
        render_app.insert_resource(ReleaseBufferSignal { rx: rx_release });

        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(CaptureLabel, CaptureDriver);
        graph.add_node_edge(bevy_render::graph::CameraDriverLabel, CaptureLabel);

        render_app
            .add_systems(ExtractSchedule, capture_extract)
            .add_systems(
                Render,
                (
                    receive_image_from_buffer.after(RenderSet::Render),
                    release_mapped_buffers.after(RenderSet::Render),
                ),
            );

        #[cfg(feature = "pixelstreaming")]
        {
            app.add_systems(
                PreUpdate,
                (handle_controller_messages.in_set(PickSet::Input),),
            );
        }
        app.add_systems(PostUpdate, handle_controllers);
    }
}

/// This system process added and removed message handlers and update controller state
/// And it process messages from Pixel Streaming
fn handle_controllers(mut controllers: Query<&mut ControllerState>) {
    for mut controller in controllers.iter_mut() {
        let controller = controller.as_mut();
        match controller {
            ControllerState::None => {}
            #[cfg(feature = "pixelstreaming")]
            ControllerState::PSControllerState(ue_controller_state) => {
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
#[cfg(feature = "pixelstreaming")]
fn handle_controller_messages(
    mut controllers: Query<(&Camera, &mut ControllerState)>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    #[cfg(feature = "pixelstreaming")] ps_conversions: PSConversions,
    mut mouse_motion_event: EventWriter<MouseMotion>,
    mut mouse_button_input_events: EventWriter<MouseButtonInput>,
    mut mouse_wheel_events: EventWriter<MouseWheel>,
    mut window_events: EventWriter<WindowEvent>,
    mut keyboard_input_events: EventWriter<KeyboardInput>,
) {
    let window = windows.single().unwrap().0;

    for (camera, mut controller) in controllers.iter_mut() {
        let controller = controller.as_mut();
        match controller {
            ControllerState::None => {}
            #[cfg(feature = "pixelstreaming")]
            ControllerState::PSControllerState(ue_controller_state) => {
                for (_peer_id, handler) in ue_controller_state.handlers.iter() {
                    for ue_msg in handler.message_receiver.try_iter() {
                        match ue_msg {
                            PSMessage::MouseMove(mouse_move) => {
                                mouse_motion_event.write(MouseMotion {
                                    delta: ps_conversions.from_ps_delta(
                                        camera,
                                        mouse_move.delta_x,
                                        mouse_move.delta_y,
                                    ),
                                });
                                window_events.write(WindowEvent::CursorMoved(CursorMoved {
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
                            PSMessage::MouseDown(mouse_down) => {
                                mouse_button_input_events.write(MouseButtonInput {
                                    button: ps_conversions.ps_to_mouse_button(mouse_down.button),
                                    state: bevy_input::ButtonState::Pressed,
                                    window,
                                });
                            }
                            PSMessage::MouseUp(mouse_up) => {
                                mouse_button_input_events.write(MouseButtonInput {
                                    button: ps_conversions.ps_to_mouse_button(mouse_up.button),
                                    state: bevy_input::ButtonState::Released,
                                    window,
                                });
                            }
                            PSMessage::UiInteraction(_ui_interaction) => {}
                            PSMessage::Command(_command) => {}
                            PSMessage::KeyDown(key_down) => {
                                keyboard_input_events.write(KeyboardInput {
                                    key_code: PSKeyCode(key_down.key_code).into(),
                                    logical_key: PSKeyCode(key_down.key_code).into(),
                                    state: bevy_input::ButtonState::Pressed,
                                    repeat: key_down.is_repeat == 1,
                                    window,
                                    text: None,
                                });
                            }
                            PSMessage::KeyUp(key_up) => {
                                keyboard_input_events.write(KeyboardInput {
                                    key_code: PSKeyCode(key_up.key_code).into(),
                                    logical_key: PSKeyCode(key_up.key_code).into(),
                                    state: bevy_input::ButtonState::Released,
                                    repeat: false,
                                    window,
                                    text: None,
                                });
                            }
                            PSMessage::KeyPress(_key_press) => {}
                            PSMessage::MouseEnter => {}
                            PSMessage::MouseLeave => {}
                            PSMessage::MouseWheel(mouse_wheel) => {
                                mouse_wheel_events.write(MouseWheel {
                                    unit: bevy_input::mouse::MouseScrollUnit::Pixel,
                                    x: 0_f32,
                                    y: mouse_wheel.delta as f32 / 10.0,
                                    window,
                                });
                            }
                            PSMessage::MouseDouble(_mouse_double) => {}
                        }
                    }
                }
            }
        }
    }
}
