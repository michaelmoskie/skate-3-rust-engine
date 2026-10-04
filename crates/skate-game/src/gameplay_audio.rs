//! First-party gameplay audio sourced from the user's owned game installation.
use crate::input::ControllerInput;
use bevy::{audio::{PlaybackMode, Volume}, prelude::*};

const WHEEL_SKID: &str = "private/audio/wheel-skid.wav";

pub(crate) struct GameplayAudioPlugin;

#[derive(Resource)]
struct GameplayAudio {
    wheel_skid: Handle<AudioSource>,
    voice: Option<Entity>,
}

impl Plugin for GameplayAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load)
            .add_systems(Update, sync.after(crate::input::poll_controllers));
    }
}

fn load(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(GameplayAudio {
        wheel_skid: assets.load(WHEEL_SKID),
        voice: None,
    });
}

fn sync(mut commands: Commands, input: Res<ControllerInput>, mut audio: ResMut<GameplayAudio>) {
    let raw = input.raw_input();
    let stick = raw.left[0].hypot(raw.left[1]);
    let intensity = raw.triggers[1].max(stick).clamp(0.0, 1.0);
    match (audio.voice, intensity > 0.15) {
        (None, true) => {
            audio.voice = Some(commands.spawn((
                AudioPlayer::new(audio.wheel_skid.clone()),
                PlaybackSettings {
                    mode: PlaybackMode::Loop,
                    volume: Volume::Linear(0.12 + intensity * 0.28),
                    speed: 0.75 + intensity * 0.75,
                    ..default()
                },
            )).id());
        }
        (Some(entity), false) => {
            commands.entity(entity).despawn();
            audio.voice = None;
        }
        _ => {}
    }
}
