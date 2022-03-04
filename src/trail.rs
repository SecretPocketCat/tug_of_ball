use bevy::{math::Vec2, prelude::*};
use bevy_prototype_lyon::prelude::*;
use bevy_time::{ScaledTime, ScaledTimeDelta};

pub struct TrailPoint(Vec2, f64);

#[derive(Component)]
pub struct Trail {
    pub(crate) points: Vec<TrailPoint>,
    pub(crate) transform_e: Entity,
    pub(crate) duration_sec: f32,
    pub(crate) max_width: f32,
}

#[derive(Component, Default)]
pub struct FadeOutTrail {
    pub(crate) decrease_duration_by: f32,
    pub(crate) stop_trail: bool,
}

pub struct TrailPlugin;
impl Plugin for TrailPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system_to_stage(CoreStage::PostUpdate, store_path_points)
            .add_system_to_stage(CoreStage::Last, draw_trail)
            .add_system(fadeout_trail);
    }
}

fn store_path_points(
    mut path_q: Query<(Entity, &mut Trail, Option<&FadeOutTrail>)>,
    transform_q: Query<&GlobalTransform>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (e, mut trail, fadeout) in path_q.iter_mut() {
        let curr_time = time.seconds_since_startup();
        let mut stop = false;

        if let Some(fadeout) = fadeout {
            stop = fadeout.stop_trail;
        }

        if trail.duration_sec > 0. && !stop {
            if let Ok(t) = transform_q.get(trail.transform_e) {
                let new_pos = t.translation.truncate();
                let mut add_point = true;

                if let Some(mut last_point) = trail.points.last_mut() {
                    if last_point.0 == new_pos {
                        last_point.1 = curr_time;
                        add_point = false;
                    }
                }

                if add_point {
                    trail.points.push(TrailPoint(new_pos, curr_time));
                }
            }
        }

        let duration = trail.duration_sec as f64;
        trail.points.drain_filter(|p| p.1 + duration < curr_time);

        if trail.points.len() == 0 {
            commands.entity(e).despawn_recursive();
        }
    }
}

fn draw_trail(mut path_q: Query<(&mut Path, &mut Trail)>, time: Res<Time>) {
    for (mut path, trail) in path_q.iter_mut() {
        if trail.points.len() > 1 {
            let mut path_builder = PathBuilder::new();
            let last = trail.points.last().unwrap();
            let trail_dur = last.1 - trail.points[0].1;
            let mut points_back = Vec::with_capacity(trail.points.len());

            // nice2have: the offset points should be angled (vertical movement breaks this right now, but that doesn't matter for the ball)
            for (i, p) in trail.points.iter().rev().enumerate() {
                let time_delta = time.seconds_since_startup() - p.1;
                let w = (1. - (time_delta / trail_dur as f64)).clamp(0., 1.)
                    * (trail.max_width as f64 / 2.);
                let pos = p.0 + Vec2::Y * w as f32;

                if i == 0 {
                    path_builder.move_to(pos);
                } else {
                    path_builder.line_to(pos);
                }

                if w == 0. {
                    break;
                }

                points_back.push(p.0 - Vec2::Y * w as f32);
            }

            for p in points_back.iter().rev() {
                path_builder.line_to(*p);
            }

            path_builder.close();
            let line = path_builder.build();
            path.0 = line.0;
        }
    }
}

fn fadeout_trail(mut path_q: Query<(&FadeOutTrail, &mut Trail)>, time: ScaledTime) {
    for (fade, mut trail) in path_q.iter_mut() {
        trail.duration_sec =
            (trail.duration_sec - fade.decrease_duration_by * time.scaled_delta_seconds()).max(0.);
    }
}
