use bevy::{
    app::ScheduleRunnerPlugin, 
    prelude::*, 
    render::RenderPlugin, 
    winit::WinitPlugin,
};
use bevy_streaming::{livekit::{LiveKitEncoder, LiveKitSettings}, StreamerCameraBuilder, StreamerHelper};
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .build()
                .disable::<WinitPlugin>()
                // Make sure pipelines are ready before rendering
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                }),
            ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 60.0)),
        ))
        .add_plugins(bevy_streaming::StreamerPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, rotate_camera))
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct SpectatorCamera;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut helper: StreamerHelper<LiveKitEncoder>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.2, 0.2),
            ..default()
        })),
        Transform::from_xyz(-2.0, 0.5, 0.0),
    ));
    
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.2, 0.8),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Player,
    ));
    
    commands.spawn((
        PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    
    // Player camera with LiveKit streaming
    let livekit_settings = LiveKitSettings {
        url: std::env::var("LIVEKIT_URL")
            .expect("LIVEKIT_URL must be set"),
        api_key: std::env::var("LIVEKIT_API_KEY")
            .expect("LIVEKIT_API_KEY must be set"),
        api_secret: std::env::var("LIVEKIT_API_SECRET")
            .expect("LIVEKIT_API_SECRET must be set"),
        room_name: std::env::var("LIVEKIT_ROOM_NAME")
            .unwrap_or_else(|_| "bevy_streaming_demo".to_string()),
        participant_identity: std::env::var("LIVEKIT_PARTICIPANT_IDENTITY")
            .unwrap_or_else(|_| "bevy_player_camera".to_string()),
        participant_name: std::env::var("LIVEKIT_PARTICIPANT_NAME")
            .unwrap_or_else(|_| "Player Camera".to_string()),
        width: 1280,
        height: 720,
        enable_controller: false,
    };
    
    commands.spawn((
        helper.new_streamer_camera(livekit_settings),
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    
    // Spectator camera with LiveKit streaming (different participant)
    let spectator_settings = LiveKitSettings {
        url: std::env::var("LIVEKIT_URL")
            .expect("LIVEKIT_URL must be set"),
        api_key: std::env::var("LIVEKIT_API_KEY")
            .expect("LIVEKIT_API_KEY must be set"),
        api_secret: std::env::var("LIVEKIT_API_SECRET")
            .expect("LIVEKIT_API_SECRET must be set"),
        room_name: std::env::var("LIVEKIT_ROOM_NAME")
            .unwrap_or_else(|_| "bevy_streaming_demo".to_string()),
        participant_identity: "bevy_spectator_camera".to_string(),
        participant_name: "Spectator Camera".to_string(),
        width: 1280,
        height: 720,
        enable_controller: false,
    };
    
    commands.spawn((
        helper.new_streamer_camera(spectator_settings),
        Camera3d::default(),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        SpectatorCamera,
    ));
}

fn move_player(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    for mut transform in query.iter_mut() {
        // Auto-move the player in a circle pattern
        let elapsed = time.elapsed_secs();
        let radius = 3.0;
        
        transform.translation.x = radius * elapsed.cos();
        transform.translation.z = radius * elapsed.sin();
        transform.translation.y = 0.5 + (elapsed * 2.0).sin() * 0.5; // Slight bobbing
    }
}

fn rotate_camera(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<SpectatorCamera>)>,
    mut camera_query: Query<&mut Transform, With<SpectatorCamera>>,
) {
    if let Ok(player_transform) = player_query.single() {
        for mut camera_transform in camera_query.iter_mut() {
            // Make spectator camera orbit around and look at the player
            let angle = time.elapsed_secs() * 0.5;
            let radius = 8.0;
            let height = 6.0;
            
            camera_transform.translation = Vec3::new(
                angle.cos() * radius,
                height,
                angle.sin() * radius,
            );
            
            camera_transform.look_at(player_transform.translation, Vec3::Y);
        }
    }
}