use crate::{
    input_binding::{InputAction, PlayerInput},
    physics::PhysLayer,
    player::{get_swing_multiplier_clamped, Player, PlayerSwing, SWING_LABEL},
    player_action::ActionStatus,
    player_animation::{AgentAnimation, AgentAnimationData},
};
use bevy::prelude::*;
use bevy_extensions::panic_on_error;
use bevy_input::*;
use heron::CollisionLayers;

pub struct PlayerControllerPlugin;
impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(process_swing_input.label(SWING_LABEL));
    }
}

// // todo: decouple from input, just set target pos and fire an event?
// // nice2have: lerp dash
// fn process_player_input(
//     input: Res<PlayerInput>,
//     mut query: Query<(
//         &Player,
//         &mut PlayerMovement,
//         &mut PlayerDash,
//         &mut Transform,
//         &PlayerSwing,
//         &mut AgentAnimationData,
//     )>,
//     aim_q: Query<(&PlayerAim, &Parent)>,
//     net_q: Query<&GlobalTransform, With<Net>>,
//     time: ScaledTime,
//     net_offset: Res<NetOffset>,
// ) {
//     for (p_aim, parent) in aim_q.iter() {
//         if let Ok((
//             player,
//             mut player_movement,
//             mut player_dash,
//             mut player_t,
//             player_swing,
//             mut p_anim,
//         )) = query.get_mut(parent.0)
//         {
//             let dir_raw = input.get_xy_axes_raw(player.id, &InputAxis::MoveX, &InputAxis::MoveY);
//             let swing_ready = matches!(player_swing.status, ActionStatus::Ready);
//             let charging = swing_ready && input.held(player.id, InputAction::Swing);
//             let speed = if charging {
//                 player_movement.charging_speed
//             } else {
//                 player_movement.speed
//             };
//             let mut dashing = false;
//             let dir = if dir_raw != Vec2::ZERO {
//                 dir_raw
//             } else {
//                 player_movement.last_non_zero_raw_dir
//             };
//             let mut move_by = (dir * speed).to_vec3();

//             if input.just_pressed(player.id, InputAction::Dash) {
//                 if let ActionStatus::Ready = player_dash.status {
//                     let dir = dir_raw.normalize_or_zero();
//                     player_dash.status = ActionStatus::Active(if dir != Vec2::ZERO {
//                         dir
//                     } else {
//                         p_aim.direction
//                     });
//                     player_dash.timer = Timer::from_seconds(player_dash.duration_sec, false);
//                     p_anim.animation = AgentAnimation::Dashing;
//                     dashing = true;
//                 }
//             }

//             if let ActionStatus::Active(dash_dir) = player_dash.status {
//                 if !player_dash.timer.finished() {
//                     move_by = (dash_dir * player_dash.speed).to_vec3();
//                     dashing = true;
//                 } else {
//                     p_anim.animation = AgentAnimation::Idle;
//                 }
//             } else if input.held(player.id, InputAction::LockPosition) {
//                 move_by = Vec3::ZERO;
//             }

//             let mut final_pos = player_t.translation + move_by * time.scaled_delta_seconds();

//             if !dashing {
//                 // easing
//                 let ease_time_delta = if dir_raw == Vec2::ZERO {
//                     -time.scaled_delta_seconds()
//                 } else {
//                     time.scaled_delta_seconds()
//                 };
//                 player_movement.easing_time += ease_time_delta;
//                 player_movement.easing_time = player_movement
//                     .easing_time
//                     .clamp(0., player_movement.time_to_max_speed);

//                 let ease_t = inverse_lerp(
//                     0.,
//                     player_movement.time_to_max_speed,
//                     player_movement.easing_time,
//                 );
//                 final_pos = player_t.translation.lerp(final_pos, ease_t);
//             } else {
//                 player_movement.easing_time = player_movement.time_to_max_speed;
//             }

//             // nice2have: get/store properly
//             let player_size = Vec2::splat(80.);
//             let is_left = player.is_left();
//             // nice2have: get (from resource or component)
//             let player_area_size = if is_left {
//                 Vec2::new(WIN_WIDTH / 2. + net_offset.0, WIN_HEIGHT)
//             } else {
//                 Vec2::new(WIN_WIDTH / 2. - net_offset.0, WIN_HEIGHT)
//             };
//             let pos_offset = Vec3::new(player_area_size.x / 2., 0., 0.);
//             let player_area_pos = if is_left {
//                 Vec3::X * net_offset.0 - pos_offset
//             } else {
//                 Vec3::X * net_offset.0 + pos_offset
//             };

//             let coll = collide(final_pos, player_size, player_area_pos, player_area_size);

//             if coll.is_some() {
//                 player_movement.easing_time = 0.;
//                 player_movement.last_non_zero_raw_dir = Vec2::ZERO;

//                 // nice2have: using colliders would probably make more sense
//                 // need to handle side coll in case the player gets pushed by a moving net

//                 if let Ok(net_t) = net_q.get_single() {
//                     let player_x = player_t.translation.x;
//                     let player_half_w = player_size.x / 2.;
//                     let net_x = net_t.translation.x;

//                     if is_left && (player_x + player_half_w) > net_x {
//                         player_t.translation.x = net_x - player_half_w;
//                     } else if !is_left && (player_x - player_half_w) < net_x {
//                         player_t.translation.x = net_x + player_half_w;
//                     }
//                 }

//                 if p_anim.animation != AgentAnimation::Idle {
//                     p_anim.animation = AgentAnimation::Idle;
//                 }

//                 trace!("{}: {:?}", if is_left { "LeftP" } else { "RightP" }, coll);
//             } else {
//                 if (final_pos - player_t.translation).length().abs() > 0.1 {
//                     if !dashing {
//                         if charging && p_anim.animation != AgentAnimation::Walking {
//                             p_anim.animation = AgentAnimation::Walking;
//                         } else if !charging && p_anim.animation != AgentAnimation::Running {
//                             p_anim.animation = AgentAnimation::Running;
//                         }
//                     }
//                 } else if p_anim.animation != AgentAnimation::Idle {
//                     p_anim.animation = AgentAnimation::Idle;
//                 }

//                 player_t.translation = final_pos;

//                 if dir_raw != Vec2::ZERO {
//                     player_movement.last_non_zero_raw_dir = dir_raw;
//                 }
//             }
//         }
//     }
// }

// nice2have: on swing down cancel prev swing?
fn process_swing_input(input: Res<PlayerInput>, mut query: Query<(&Player, &mut PlayerSwing)>) {
    for (player, mut player_swing) in query.iter_mut() {
        if let Some(ActionState::Released(key_data)) =
            input.get_button_action_state(player.id, &InputAction::Swing)
        {
            if let ActionStatus::Ready = player_swing.status {
                player_swing.status =
                    ActionStatus::Active(get_swing_multiplier_clamped(key_data.duration));
                player_swing.timer = Timer::from_seconds(player_swing.duration_sec, false);
            }
        }
    }
}
