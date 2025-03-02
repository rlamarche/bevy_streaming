use bevy::{
    app::{RunMode, ScheduleRunnerPlugin},
    prelude::*,
    render::RenderPlugin,
    time::TimeUpdateStrategy,
    winit::WinitPlugin,
};
use bevy_streaming::{
    CongestionControl, SignallingServer, StreamerHelper, StreamerPlugin, StreamerSettings,
};
use camera_controller::{CameraController, CameraControllerPlugin};
use std::time::Duration;

mod camera_controller;

fn main() -> AppExit {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins
            .build()
            // Disable the WinitPlugin to prevent the creation of a window
            .disable::<WinitPlugin>()
            // Make sure pipelines are ready before rendering
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            }),
        // Add the ScheduleRunnerPlugin to run the app in loop mode
        ScheduleRunnerPlugin {
            run_mode: RunMode::Loop { wait: None },
        },
        StreamerPlugin,
        CameraControllerPlugin,
    ));

    // Update the time at a fixed rate of 60 FPS
    // app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
    //     1.0 / 60.0,
    // )));

    // Setup
    app.add_systems(Startup, (setup_cameras, setup_scene));

    // Run the app
    app.run()
}

fn setup_cameras(mut commands: Commands, mut streamer: StreamerHelper) {
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        streamer.new_streamer_camera(StreamerSettings {
            signalling_server: SignallingServer::UePixelStreaming {
                uri: "ws://localhost:8888".to_string(),
                streamer_id: Some("simple".to_string()),
            },
            // signalling_server: SignallingServer::GstWebRtc {
            //     uri: "ws://127.0.0.1:8443".to_string(),
            //     peer_id: None,
            // },
            width: 1920,
            height: 1080,
            video_caps: Some("video/x-h264".to_string()),
            congestion_control: Some(CongestionControl::Disabled),
        }),
        CameraController::default(),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(2.5, 8.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        streamer.new_streamer_camera(StreamerSettings {
            signalling_server: SignallingServer::UePixelStreaming {
                uri: "ws://localhost:8888".to_string(),
                streamer_id: Some("simple/2".to_string()),
            },
            width: 1920,
            height: 1080,
            video_caps: Some("video/x-h264".to_string()),
            congestion_control: Some(CongestionControl::Disabled),
        }),
        // CameraController::default(),
    ));
}

/// set up a simple 3D scene
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}
