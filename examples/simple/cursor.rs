use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_window::WindowEvent;

pub(crate) struct CursorPlugin;

#[derive(Component)]
struct Cursor {}

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(PreUpdate, update_cursor_camera)
            .add_systems(Update, update_cursor_position);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_window: Query<&Window, With<PrimaryWindow>>,
) {
    let cursor_image = ImageNode::new(asset_server.load("cursors/normal.png"));

    let mut spawnpos = (0.0, 0.0);

    if let Some(position) = q_window.single().cursor_position() {
        spawnpos = (position.x, position.y);
    }

    commands.spawn((
        cursor_image,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(spawnpos.1),
            left: Val::Px(spawnpos.0),
            ..default()
        },
        Cursor {},
    ));
}

fn update_cursor_camera(
    mut commands: Commands,
    q_camera: Query<Entity, With<Camera>>,
    q_cursor: Query<Entity, (With<Cursor>, Without<TargetCamera>)>,
) {
    if let Some(cursor_entity) = q_cursor.iter().next() {
        if let Some(camera_entity) = q_camera.iter().next() {
            commands
                .entity(cursor_entity)
                .insert(TargetCamera(camera_entity));
        }
    }
}

fn update_cursor_position(
    mut q_cursor: Query<&mut Node, With<Cursor>>,
    mut window_events: EventReader<WindowEvent>,
) {
    let mut cursor = q_cursor.single_mut();

    if let Some(WindowEvent::CursorMoved(cursor_moved)) = window_events
        .read()
        .filter(|event| matches!(event, WindowEvent::CursorMoved(..)))
        .last()
    {
        let cursor = cursor.as_mut();
        cursor.top = Val::Px(cursor_moved.position.y);
        cursor.left = Val::Px(cursor_moved.position.x);
    }
}
