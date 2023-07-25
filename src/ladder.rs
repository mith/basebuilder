use bevy::prelude::*;
use bevy::sprite::SpriteBundle;
use bevy_rapier2d::prelude::{Collider, CollisionGroups, Group, RigidBody};

use crate::{
    climbable::Climbable,
    labor::build_structure::{ConstructionCompletedEvent, CONSTRUCTION_COLLISION_GROUP},
};

pub struct LadderPlugin;

impl Plugin for LadderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, finish_ladder_construction);
    }
}

#[derive(Component)]
pub struct Ladder;

pub const LADDER_COLLISION_GROUP: Group = Group::GROUP_6;

pub fn spawn_ladder(commands: &mut Commands, asset_server: &AssetServer, position: Vec3) -> Entity {
    commands
        .spawn((
            Ladder,
            SpriteBundle {
                texture: asset_server.load("textures/ladder.png"),
                sprite: Sprite {
                    color: Color::rgba(1.0, 1.0, 1.0, 0.5),
                    ..default()
                },
                transform: Transform::from_translation(position),
                ..default()
            },
            RigidBody::Fixed,
            Collider::cuboid(5., 16.),
            CollisionGroups::new(CONSTRUCTION_COLLISION_GROUP, Group::empty()),
        ))
        .id()
}

fn finish_ladder_construction(
    mut commands: Commands,
    mut construction_complete_events: EventReader<ConstructionCompletedEvent>,
    mut ladder_query: Query<&mut Sprite, With<Ladder>>,
) {
    for event in construction_complete_events.iter() {
        if let Ok(mut ladder_sprite) = ladder_query.get_mut(event.construction_site) {
            ladder_sprite.color = Color::WHITE;
            commands.entity(event.construction_site).insert((
                Climbable,
                CollisionGroups::new(LADDER_COLLISION_GROUP, Group::all()),
            ));
        }
    }
}
