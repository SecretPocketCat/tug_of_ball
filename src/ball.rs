use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
use bevy_input::ActionInput;
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};
use heron::rapier_plugin::PhysicsWorld;
use heron::*;

use crate::{player::{PlayerSwing, ActionStatus, PlayerMovement, Player}, PlayerInput, InputAxis, wall::Wall, WIN_WIDTH};

const BALL_SIZE: f32 = 30.;

#[derive(Component, Inspectable)]
pub struct Ball {
    dir: Vec2,
    size: f32,
}

impl Default for Ball {
    fn default() -> Self {
        Self {
            size: BALL_SIZE,
            dir: Default::default(),
        }
    }
}

pub struct BallPlugin;
impl Plugin for BallPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_startup_system(setup)
            .add_system(movement)
            .add_system_to_stage(CoreStage::PostUpdate, handle_collisions);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn_bundle(SpriteBundle {
        transform: Transform::from_xyz( -WIN_WIDTH / 2. + 250., 0., 0.),
        texture: asset_server.load("icon.png"),
        sprite: Sprite {
            color: Color::BLACK,
            custom_size: Some(Vec2::splat(BALL_SIZE)),
            ..Default::default()
        },
        ..Default::default()
    }).insert(Ball::default())
    .insert(RigidBody::KinematicPositionBased)
    .insert(CollisionShape::Sphere {
        radius: 15.,
    })
    .insert(Name::new("Ball"));
}

fn movement(
    mut query: Query<(&mut Ball, &mut Transform)>,
    time: ScaledTime,
) {
    for (mut ball, mut t) in query.iter_mut() {
        // very simple drag
        ball.dir *= 1. - 1. * time.scaled_delta_seconds();

        // todo: simulate bounces
        // let bounce = time.time.seconds_since_startup().sin() as f32 * 10. * ball.dir.length() * Vec3::Y;
        // info!("bounce {}", bounce);

        t.translation += ball.dir.to_vec3() * 800. * time.scaled_delta_seconds() /*+ bounce*/;
    }
}

// todo: 'dashing swing'?
fn handle_collisions(
    mut coll_events: EventReader<CollisionEvent>,
    input: Res<PlayerInput>,
    mut ball_q: Query<(&mut Ball, &Transform)>,
    mut player_q: Query<(&Player, &mut PlayerSwing)>,
    wall_q: Query<&Sprite, With<Wall>>,
    time: ScaledTime,
) {
    for ev in coll_events.iter() {
        if ev.is_started() {
            let mut ball;
            let ball_t;
            let other_e;
            let (entity_1, entity_2) = ev.rigid_body_entities();
            if let Ok(b) = ball_q.get_mut(entity_1) {
                ball = b.0;
                ball_t = b.1;
                other_e = entity_2;
            } else if let Ok(b) = ball_q.get_mut(entity_2) {
                ball = b.0;
                ball_t = b.1;
                other_e = entity_1;
            } else {
                continue;
            }

            if let Ok((player, mut swing)) = player_q.get_mut(other_e) {
                if let ActionStatus::Active(ball_speed_multiplier) = swing.status {
                    if !swing.timer.finished() {
                        swing.start_cooldown();
                        // todo: limit angle to roughly 45deg?
                        let dir = input.get_xy_axes(player.id, &InputAxis::X, &InputAxis::Y);
                        ball.dir = dir * ball_speed_multiplier;
                    }
                }
            }
            else if let Ok(wall_sprite) = wall_q.get(other_e) {
                let size = wall_sprite.custom_size.unwrap();
                let is_hor = size.x > size.y;
                let x = if is_hor { 1. } else { -1. }; 
                ball.dir *= Vec2::new(x, -x);
            }
        }
    }
}
