use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
use bevy_inspector_egui::Inspectable;
use heron::*;

use crate::{WIN_WIDTH, WIN_HEIGHT, player::PlayerScore};

#[derive(Component, Inspectable)]
struct PointsText;

#[derive(Component, Inspectable)]
struct GamesText;

pub struct ScorePlugin;
impl Plugin for ScorePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_startup_system(setup)
            .add_system(update_score_ui);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // todo: center align
    commands
        .spawn_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::Center,
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(5.0),
                    right: Val::Px(15.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            text: Text::with_section(
                "",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 100.0,
                    color: Color::WHITE,
                },
                // Note: You can use `Default::default()` in place of the `TextAlignment`
                TextAlignment {
                    horizontal: HorizontalAlign::Center,
                    ..Default::default()
                },
            ),
            ..Default::default()
        })
        .insert(PointsText);

    commands
        .spawn_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::Center,
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(5.0),
                    left: Val::Px(15.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            text: Text::with_section(
                "",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 100.0,
                    color: Color::WHITE,
                },
                // Note: You can use `Default::default()` in place of the `TextAlignment`
                TextAlignment {
                    horizontal: HorizontalAlign::Center,
                    ..Default::default()
                },
            ),
            ..Default::default()
        })
        .insert(GamesText);
}

fn update_score_ui(
    score_q: Query<(&PlayerScore, ChangeTrackers<PlayerScore>)>,
    mut points_text_q: Query<&mut Text, (With<PointsText>, Without<GamesText>)>,
    mut games_text_q: Query<&mut Text, (With<GamesText>, Without<PointsText>)>,
) {
    let any_changes = score_q
        .iter()
        .any(|(_, t)| { t.is_changed() });

    if any_changes {
        // nice2have: deuce, advantage, game & all that jazz
        points_text_q.single_mut().sections[0].value = score_q
            .iter()
            .map(|(s, _)| {
                match s.points {
                    // nice2have: proper love/heart
                    0 => String::from("<3"),
                    1..=2 => (s.points * 15).to_string(),
                    3 => String::from("40"),
                    4 => String::from("ADV"),
                    _ => (37 + s.points).to_string(),
                }
            })
            .collect::<Vec<String>>()
            .join(" : ");

        games_text_q.single_mut().sections[0].value = score_q
            .iter()
            .map(|(s, _)| s.games.to_string())
            .collect::<Vec<String>>()
            .join(" : ");
    }
}
