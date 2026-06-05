use bevy::{
    camera::ScalingMode,
    prelude::*,
    window::{EnabledButtons, WindowResolution},
};

use bevy_spritesheet_animation::prelude::*;

const U_GAME_WIDTH: u32 = 1280;
const U_GAME_HEIGHT: u32 = 720;

const F_GAME_WIDTH: f32 = 1280.0;
const F_GAME_HEIGHT: f32 = 720.0;

#[derive(Component)]
struct Dying;

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component, Default)]
pub struct MoveTarget {
    pub position: Vec3,
    pub active: bool,
}

#[derive(Component)]
struct UnitBaseStats {
    health: f32,
    max_health: f32,
    speed: f32
}
#[derive(Component)]
struct Player;

#[derive(Resource)]
struct SpriteAnimationHandle {
    _base_sprite: Sprite,
    _base_animation: Handle<Animation>,
    death_sprite: Sprite,
    death_animation: Handle<Animation>
}

// enum UnitFaction {
//     IronCorps,
//     DuskFleet,
//     Biomantes,
// }

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(U_GAME_WIDTH, U_GAME_HEIGHT),
                        resizable: false,
                        enabled_buttons: EnabledButtons {
                            maximize: true,
                            minimize: true,
                            close: true,
                        },
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(SpritesheetAnimationPlugin)
        .add_systems(Startup, (camera_setup, load_biomantes_scout_base))
        .add_systems(Update, (update_system, handle_death_animation, camera_input_system, player_input_system, player_move_system))
        .run();
}

fn camera_setup(mut commands: Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed {
        width: F_GAME_WIDTH,
        height: F_GAME_HEIGHT,
    };

    commands.spawn((Camera2d, Projection::Orthographic(projection)));
}

fn camera_input_system(
    kb: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera: Single<&mut Transform, With<Camera>>,
) {
    const SPEED: f32 = 400.0;

    let mut t = camera.into_inner();

    if kb.pressed(KeyCode::KeyW) {
        t.translation.y += SPEED * time.delta_secs();
    }

    if kb.pressed(KeyCode::KeyA) {
        t.translation.x -= SPEED * time.delta_secs();
    }

    if kb.pressed(KeyCode::KeyS) {
        t.translation.y -= SPEED * time.delta_secs();
    }

    if kb.pressed(KeyCode::KeyD) {
        t.translation.x += SPEED * time.delta_secs();
    }
}

fn player_input_system(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    player: Single<&mut MoveTarget, With<Player>>,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }

    let window = *window;
    let (camera, camera_transform) = *camera;

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(world_pos) =
        camera.viewport_to_world_2d(camera_transform, cursor_pos)
    else {
        return;
    };

    let mut target = player.into_inner();

    target.position = world_pos.extend(0.0);
    target.active = true;
}

fn player_move_system(
    time: Res<Time>,
    player: Single<(&mut Transform, &mut MoveTarget), With<Player>>,
) {
    const SPEED: f32 = 200.0;

    let (mut transform, mut target) = player.into_inner();

    if !target.active {
        return;
    }

    let dir = target.position - transform.translation;
    let distance = dir.length();

    if distance < 2.0 {
        target.active = false;
        return;
    }

    transform.translation +=
        dir.normalize() * SPEED * time.delta_secs();
}

fn load_biomantes_scout_base(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut animations: ResMut<Assets<Animation>>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {

    let base_image = assets.load("unit_biomantes_scout_base.png");
    
    let base_layout = TextureAtlasLayout::from_grid(UVec2::new(64, 64), 7, 1, None, None);
    let base_layout_handle = atlas_layouts.add(base_layout);
    
    let base_animation = Spritesheet::new(&base_image, 7, 1)
        .create_animation()
        .add_row(0)
        .set_duration(AnimationDuration::PerFrame(100))
        .set_repetitions(AnimationRepeat::Loop)
        .build();
    let base_handle = animations.add(base_animation);

    let death_image = assets.load("unit_biomantes_scout_destruction.png");
    let death_layout = TextureAtlasLayout::from_grid(UVec2::new(64, 64), 9, 1, None, None);
    let death_layout_handle = atlas_layouts.add(death_layout);
    
    let death_animation = Spritesheet::new(&death_image, 9, 1)
        .create_animation()
        .add_row(0)
        .set_duration(AnimationDuration::PerFrame(100))
        .set_repetitions(AnimationRepeat::Times(1))
        .build();
    let death_handle = animations.add(death_animation);
    
    let sprite = Sprite {
        image: base_image.into(),
        texture_atlas: Some(TextureAtlas {
            layout: base_layout_handle.clone(),
            index: 0,
        }),
        color: Color::WHITE,
        ..default()
    };

    let sprite_death = Sprite {
        image: death_image.into(),
        texture_atlas: Some(TextureAtlas {
            layout: death_layout_handle.clone(),
            index: 0,
        }),
        color: Color::WHITE,
        ..default()
    };

    commands.spawn((
        sprite.clone(),
        SpritesheetAnimation::new(base_handle.clone()),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Player, MoveTarget::default()
    ));

    commands.insert_resource(SpriteAnimationHandle {
        _base_sprite: sprite,
        _base_animation: base_handle,
        death_sprite: sprite_death,
        death_animation: death_handle,
    }); 
}

fn update_system(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut query: Query<(Entity, &mut SpritesheetAnimation, &mut MoveTarget), Without<Dying>>,
    animations: Res<SpriteAnimationHandle>
) {
    for (ent, mut anim, mut m) in query.iter_mut() {
        if mouse.pressed(MouseButton::Right) {
            // MoveTarget = MouseWorldPosition
        }

        if keyboard.just_pressed(KeyCode::KeyK) {
            commands.entity(ent).insert(animations.death_sprite.clone()).insert(Dying);
            anim.switch(animations.death_animation.clone());
        }
    }
}

fn handle_death_animation(
    mut commands: Commands,
    mut messages: MessageReader<AnimationEvent>,
    query: Query<Entity, With<Dying>>,
) {
    for event in messages.read() {
        if let AnimationEvent::AnimationEnd { entity, .. } = event {
            if query.contains(*entity) {
                commands.entity(*entity).despawn();
            }
        }
    }
}

