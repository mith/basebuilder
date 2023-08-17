mod pathfinding;
pub mod terrain;

use bevy::prelude::*;
#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::render::{DebugRenderContext, RapierDebugRenderPlugin};

use crate::pan_zoom_camera2d::{PanZoomCamera2d, PanZoomCamera2dBundle};

use self::{
    pathfinding::{PathfindingDebugPlugin, PathfindingDebugState},
    terrain::TerrainDebugPlugin,
};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "inspector")]
        app.add_plugins(WorldInspectorPlugin::default().run_if(resource_exists::<Inspector>()));

        app.add_plugins(RapierDebugRenderPlugin {
            enabled: false,
            ..default()
        })
        .add_systems(
            Update,
            (
                toggle_physics_debug,
                toggle_freelook_camera,
                toggle_inspector,
                toggle_pathfinding_debug,
            )
                .in_set(DebugSet),
        )
        .add_plugins((PathfindingDebugPlugin, TerrainDebugPlugin));
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub struct DebugSet;

#[derive(Resource)]
struct Inspector;

fn toggle_inspector(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    maybe_inspector: Option<Res<Inspector>>,
) {
    if keyboard_input.just_pressed(KeyCode::F3) {
        if maybe_inspector.is_some() {
            info!("Disabling inspector");
            commands.remove_resource::<Inspector>();
        } else {
            info!("Enabling inspector");
            commands.insert_resource(Inspector);
        }
    }
}

fn toggle_physics_debug(
    mut debug_render: ResMut<DebugRenderContext>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::F2) {
        info!("Toggling physics debug");
        debug_render.enabled = !debug_render.enabled;
    }
}

fn spawn_freelook_camera(mut commands: Commands) {
    commands.spawn(PanZoomCamera2dBundle {
        camera: Camera2dBundle {
            camera: Camera {
                order: 100,
                ..default()
            },
            ..default()
        },
        ..default()
    });
}

fn toggle_freelook_camera(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    query: Query<Entity, With<PanZoomCamera2d>>,
) {
    if keyboard_input.just_pressed(KeyCode::F1) {
        if let Ok(entity) = query.get_single() {
            commands.entity(entity).despawn_recursive();
        } else {
            spawn_freelook_camera(commands);
        }
    }
}

fn toggle_pathfinding_debug(
    keyboard_input: Res<Input<KeyCode>>,
    current_state: Res<State<PathfindingDebugState>>,
    mut next_state: ResMut<NextState<PathfindingDebugState>>,
) {
    if keyboard_input.just_pressed(KeyCode::F4) {
        match current_state.get() {
            PathfindingDebugState::Enabled => {
                info!("Disabling pathfinding debug");
                next_state.set(PathfindingDebugState::Disabled);
            }
            PathfindingDebugState::Disabled => {
                info!("Enabling pathfinding debug");
                next_state.set(PathfindingDebugState::Enabled);
            }
        }
    }
}
