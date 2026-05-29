use bevy::{
    camera::ScalingMode, prelude::*, window::{EnabledButtons, WindowResolution}
};

const U_GAME_WIDTH: u32 = 1280;
const U_GAME_HEIGHT: u32 = 720;

const F_GAME_WIDTH: f32 = 1280.0;
const F_GAME_HEIGHT: f32 = 720.0;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: WindowResolution::new(U_GAME_WIDTH, U_GAME_HEIGHT),
            resizable: false,
            enabled_buttons: EnabledButtons {
                maximize: true,
                minimize: true,
                close: true,
            },
            ..Default::default()
        }),
        ..Default::default()
    }));
    app.add_systems(Startup, camera_setup);
    app.run();
}

fn camera_setup(mut commands: Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed {
        width: F_GAME_WIDTH,
        height: F_GAME_HEIGHT,
    };

    commands.spawn((Camera2d, Projection::Orthographic(projection)));
}
