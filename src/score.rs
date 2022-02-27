use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
use bevy_inspector_egui::Inspectable;
use heron::*;

use crate::{WIN_WIDTH, WIN_HEIGHT, player::PlayerScore};

#[derive(Component, Inspectable)]
struct ScoreText;

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
    commands
        .spawn_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                position_type: PositionType::Absolute,
                position: Rect {
                    bottom: Val::Px(5.0),
                    right: Val::Px(15.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            text: Text::with_section(
                "0:0",
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
        .insert(ScoreText);
}

fn update_score_ui(
    score_q: Query<(&PlayerScore, ChangeTrackers<PlayerScore>)>,
    mut text_q: Query<&mut Text, With<ScoreText>>,
) {
    let any_changes = score_q
        .iter()
        .any(|(_, t)| { t.is_changed() });

    if any_changes {
        let mut ui_txt = text_q.single_mut();
        // todo: deuce, advantage, game & all that jazz
        ui_txt.sections[0].value = score_q
            .iter()
            .map(|(s, _)| {
                match s.score {
                    // todo: proper love/heart
                    0 => String::from("<3"),
                    1..=2 => (s.score * 15).to_string(),
                    3 => String::from("40"),
                    _ => (37 + s.score).to_string(),
                }
            })
            .collect::<Vec<String>>()
            .join(" : ");
    }
}
