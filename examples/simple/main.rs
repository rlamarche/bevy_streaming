use bevy::{
    app::ScheduleRunnerPlugin, prelude::*, render::RenderPlugin, time::TimeUpdateStrategy,
    winit::WinitPlugin,
};
use bevy_streaming::{
    CongestionControl, SignallingServer, StreamerHelper, StreamerPlugin, StreamerSettings,
};
use camera_controller::{CameraController, CameraControllerPlugin};
use cursor::CursorPlugin;
use std::time::Duration;

mod camera_controller;
mod cursor;

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
        ScheduleRunnerPlugin::run_loop(
            // Run 60 times per second.
            Duration::from_secs_f64(1.0 / 60.0),
        ),
        StreamerPlugin,
        CameraControllerPlugin,
        CursorPlugin,
    ));

    // Update the time at a fixed rate of 60 FPS
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));

    // Setup
    app.add_systems(Startup, (setup_cameras, setup_scene));

    app.add_systems(Update, update_player_position_and_spectator_view);

    // Run the app
    app.run()
}

#[derive(Component)]
struct PlayerCamera;

#[derive(Component)]
struct SpectatorCamera;

fn setup_cameras(mut commands: Commands, mut streamer: StreamerHelper) {
    // camera
    let main_camera = commands
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
            streamer.new_streamer_camera(StreamerSettings {
                signalling_server: SignallingServer::PixelStreaming {
                    uri: "ws://localhost:8888".to_string(),
                    streamer_id: Some("player".to_string()),
                },
                // signalling_server: SignallingServer::GstWebRtc {
                //     uri: "ws://127.0.0.1:8443".to_string(),
                //     peer_id: None,
                // },
                width: 1920,
                height: 1080,
                video_caps: Some("video/x-h264".to_string()),
                congestion_control: Some(CongestionControl::Disabled),
                enable_controller: true,
            }),
            CameraController::default(),
            PlayerCamera,
        ))
        .id();

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(2.5, 12.0, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        streamer.new_streamer_camera(StreamerSettings {
            signalling_server: SignallingServer::PixelStreaming {
                uri: "ws://localhost:8888".to_string(),
                streamer_id: Some("spectator".to_string()),
            },
            width: 1920,
            height: 1080,
            video_caps: Some("video/x-h264".to_string()),
            congestion_control: Some(CongestionControl::Disabled),
            enable_controller: false,
        }),
        SpectatorCamera,
    ));

    commands.spawn((
        Text::new("You need to specify TargetCamera to display UI elements, because there is no main window."),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        UiTargetCamera(main_camera),
    ));
}

#[derive(Component)]
struct Player;

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
    // player
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(255, 0, 0))),
        Player,
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

fn update_player_position_and_spectator_view(
    mut q_player_transform: Query<&mut Transform, With<Player>>,
    q_player_camera_transform: Query<
        &Transform,
        (
            With<PlayerCamera>,
            Without<Player>,
            Without<SpectatorCamera>,
        ),
    >,
    mut q_spectator_camera_transform: Query<
        &mut Transform,
        (With<SpectatorCamera>, Without<Player>),
    >,
) {
    let camera_transform = q_player_camera_transform.single().unwrap();
    let mut player_position = q_player_transform.single_mut().unwrap();

    player_position.translation = camera_transform.translation;

    let mut spectator_camera_transform = q_spectator_camera_transform.single_mut().unwrap();
    spectator_camera_transform.look_at(camera_transform.translation, Vec3::Y);
}
