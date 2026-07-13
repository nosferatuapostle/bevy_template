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

#[derive(Resource)]
struct GameAssets {
    units: UnitAssets,
    projectiles: ProjectileAssets,
}

struct UnitAssets {
    biomantes_scout: UnitAsset,
}

struct ProjectileAssets {
    corrosion_wave: AnimationSpriteAsset,
}

struct UnitAsset {
    base: AnimationSpriteAsset,
    engine: AnimationSpriteAsset,
}

struct AnimationSpriteAsset {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    animation: Handle<Animation>,
}

impl AnimationSpriteAsset {
    fn sprite(&self) -> Sprite {
        create_sprite(self.image.clone(), self.layout.clone())
    }

    fn ssanimation(&self) -> SpritesheetAnimation {
        SpritesheetAnimation::new(self.animation.clone())
    }
}

#[derive(Resource, Default)]
struct CursorCoords {
    screen: Vec2,
    world: Vec2,
}

#[derive(Component)]
struct Player;

#[derive(Component, Default)]
struct MoveTarget {
    target: Option<Vec2>,
}

#[derive(Component, Default)]
struct Velocity {
    linear: Vec2,
}

#[derive(Component)]
struct MaxSpeed(f32);

#[derive(Component)]
struct Unit;

#[derive(Component)]
struct Engine(Entity);

#[derive(Component)]
struct Projectile {
    direction: Vec2,
    life_time: f32,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
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
            }), // .set(ImagePlugin::default_nearest())
        )
        .add_plugins(SpritesheetAnimationPlugin)
        .add_systems(Startup, (setup, spawn_player_unit).chain())
        .add_systems(
            Update,
            (
                update,
                cursor_moved_system,
                camera_input_system,
                player_input_system,
                velocity_system,
                unit_movement_system,
                engine_system,
                projectile_movement_system,
                projectile_life_system,
            ),
        )
        // .add_systems(Update, (update_system, handle_death_animation, camera_input_system, player_input_system, player_move_system))
        .run();
}

fn setup(
    mut cmds: Commands,
    assets: Res<AssetServer>,
    mut animations: ResMut<Assets<Animation>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed {
        width: F_GAME_WIDTH,
        height: F_GAME_HEIGHT,
    };

    cmds.spawn((Camera2d, Projection::Orthographic(projection)));

    cmds.init_resource::<CursorCoords>();

    // ----------------
    // Projectile
    // ----------------

    let wave_corrosion_img = assets.load("projectile_wave_corrosion.png");

    let wave_corrosion_layout = layouts.add(create_atlas(64, 64, 6, 1));

    let wave_corrosion_animation = animations.add(create_animation(
        wave_corrosion_img.clone(),
        6,
        1,
        AnimationRepeat::Loop,
    ));

    let corrosion_wave = AnimationSpriteAsset {
        image: wave_corrosion_img,
        layout: wave_corrosion_layout,
        animation: wave_corrosion_animation,
    };

    // ----------------
    // Unit
    // ----------------

    let base_img = assets.load("unit_biomantes_scout_base.png");

    let engine_img = assets.load("unit_biomantes_scout_engine.png");

    let base = AnimationSpriteAsset {
        image: base_img.clone(),
        layout: layouts.add(create_atlas(64, 64, 7, 1)),
        animation: animations.add(create_animation(base_img, 7, 1, AnimationRepeat::Loop)),
    };

    let engine = AnimationSpriteAsset {
        image: engine_img.clone(),
        layout: layouts.add(create_atlas(64, 64, 8, 1)),
        animation: animations.add(create_animation(engine_img, 8, 1, AnimationRepeat::Loop)),
    };

    cmds.insert_resource(GameAssets {
        units: UnitAssets {
            biomantes_scout: UnitAsset { base, engine },
        },

        projectiles: ProjectileAssets { corrosion_wave },
    });
}

fn spawn_player_unit(mut cmds: Commands, game_assets: Res<GameAssets>) {
    let transform = Transform::from_xyz(0.0, 0.0, 0.0);

    let player_unit = &game_assets.units.biomantes_scout;

    let entt = cmds
        .spawn((
            Player,
            Unit,
            MoveTarget::default(),
            Velocity::default(),
            MaxSpeed(200.0),
            transform,
            player_unit.base.sprite(),
            player_unit.base.ssanimation(),
        ))
        .id();

    cmds.spawn((
        player_unit.engine.sprite(),
        player_unit.engine.ssanimation(),
        transform,
        Engine(entt),
        Visibility::Visible,
    ))
    .set_parent_in_place(entt);
}

fn update() {}

fn cursor_moved_system(
    mut events: MessageReader<CursorMoved>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut cursor: ResMut<CursorCoords>,
) {
    let (cam, gt) = *camera;

    for event in events.read() {
        cursor.screen = event.position;
        // println!("cursor screen coords: {}", cursor.screen);

        if let Ok(world_coords) = cam.viewport_to_world_2d(gt, event.position) {
            cursor.world = world_coords;
            // println!("cursor world coords: {}", cursor.world);
        }
    }
}

fn camera_input_system(
    kb: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    camera: Single<&mut Transform, With<Camera>>,
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
    mut cmds: Commands,
    game_assets: Res<GameAssets>,
    btns: Res<ButtonInput<MouseButton>>,
    cursor: Res<CursorCoords>,
    mut player: Single<(&mut MoveTarget, &mut Transform), With<Player>>,
) {
    if btns.just_pressed(MouseButton::Right) {
        player.0.target = Some(cursor.world);
    }

    if btns.just_pressed(MouseButton::Left) {
        let projectile = &game_assets.projectiles.corrosion_wave;

        let direction = cursor.world - player.1.translation.truncate();

        if direction.length_squared() > 0.0001 {
            let mut transform = Transform::from_translation(player.1.translation);
            let angle = direction.y.atan2(direction.x);
            transform.rotation = Quat::from_rotation_z(angle - std::f32::consts::FRAC_PI_2);

            cmds.spawn((
                Projectile {
                    direction: direction.normalize(),
                    life_time: 2.0,
                },
                transform,
                projectile.sprite(),
                projectile.ssanimation(),
            ));
        }
    }
}

fn velocity_system(query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in query {
        transform.translation += (velocity.linear * time.delta_secs()).extend(0.0);
    }
}

fn unit_movement_system(
    query: Query<(&mut Transform, &mut Velocity, &mut MoveTarget, &MaxSpeed), With<Unit>>,
) {
    for (mut transform, mut velocity, mut target, speed) in query {
        let Some(destination) = target.target else {
            velocity.linear = Vec2::ZERO;
            continue;
        };

        let direction = destination - transform.translation.truncate();

        if direction.length_squared() < 4.0 {
            target.target = None;
            velocity.linear = Vec2::ZERO;
            continue;
        }

        velocity.linear = direction.normalize() * speed.0;

        let angle = direction.y.atan2(direction.x);
        transform.rotation = Quat::from_rotation_z(angle - std::f32::consts::FRAC_PI_2);

        // if direction.length_squared() < 0.0001 {
        //     // move_target.stop();
        //     continue;
        // }

        // let direction = direction.normalize();

        // let vel = direction * 200.0 * time.delta_secs();

        // transform.translation.x += vel.x;
        // transform.translation.y += vel.y;
    }
}

fn projectile_movement_system(query: Query<(&mut Transform, &mut Projectile)>, time: Res<Time>) {
    for (mut transform, projectile) in query {
        transform.translation.x += projectile.direction.x * 400.0 * time.delta_secs();
        transform.translation.y += projectile.direction.y * 400.0 * time.delta_secs();
    }
}

fn projectile_life_system(
    mut cmds: Commands,
    mut query: Query<(Entity, &mut Projectile)>,
    time: Res<Time>,
) {
    for (entity, mut projectile) in &mut query {
        projectile.life_time -= time.delta_secs();

        if projectile.life_time <= 0.0 {
            cmds.entity(entity).despawn();
        }
    }
}

fn engine_system(mut q_engine: Query<(&mut Visibility, &Engine)>, q_target: Query<&MoveTarget>) {
    for (mut visibility, engine) in &mut q_engine {
        let Ok(movement) = q_target.get(engine.0) else {
            continue;
        };

        let new_visibility = if movement.target.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        if *visibility != new_visibility {
            *visibility = new_visibility;
        }
    }
}

fn create_atlas(x: u32, y: u32, columns: u32, rows: u32) -> TextureAtlasLayout {
    return TextureAtlasLayout::from_grid(UVec2::new(x, y), columns, rows, None, None);
}

fn create_animation(
    img: Handle<Image>,
    columns: usize,
    rows: usize,
    animation_repeat: AnimationRepeat,
) -> Animation {
    return Spritesheet::new(&img, columns, rows)
        .create_animation()
        .add_row(0)
        .set_repetitions(animation_repeat)
        .build();
}

fn create_sprite(img: Handle<Image>, layout: Handle<TextureAtlasLayout>) -> Sprite {
    return Sprite {
        image: img.into(),
        texture_atlas: Some(TextureAtlas {
            layout: layout,
            index: 0,
        }),
        color: Color::WHITE,
        ..default()
    };
}
