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
    destruction: AnimationSpriteAsset,
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
struct Dying;

#[derive(Component)]
struct Engine(Entity);

#[derive(Resource)]
struct CameraBounds {
    half_size: Vec2,
}

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct EnemyAI {
    timer: Timer,
}

#[derive(Resource)]
struct SpawnTimer(Timer);

#[derive(Component)]
struct Projectile {
    direction: Vec2,
    life_time: f32,
}

#[derive(Message)]
struct DeathEvent {
    entity: Entity,
    killer: Entity,
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
        .add_message::<DeathEvent>()
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
                unit_ai_system,
                unit_spawn_system,
                engine_system,
                projectile_movement_system,
                projectile_life_system,
                destruction_animation_system,
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

    cmds.insert_resource(CameraBounds {
        half_size: Vec2::new(F_GAME_WIDTH * 0.5, F_GAME_HEIGHT * 0.5),
    });

    cmds.init_resource::<CursorCoords>();

    cmds.insert_resource(SpawnTimer(Timer::from_seconds(2.0, TimerMode::Repeating)));

    // ----------------
    // Projectile
    // ----------------

    let img = assets.load("projectile_wave_corrosion.png");

    let corrosion_wave = AnimationSpriteAsset {
        image: assets.load("projectile_wave_corrosion.png"),
        layout: layouts.add(create_atlas(64, 64, 6, 1)),
        animation: animations.add(create_animation(img.clone(), 6, 1, AnimationRepeat::Loop)),
    };

    // ----------------
    // Unit
    // ----------------

    let base_img = assets.load("unit_biomantes_scout_base.png");
    let engine_img = assets.load("unit_biomantes_scout_engine.png");
    let destruction_img = assets.load("unit_biomantes_scout_destruction.png");

    let biomantes_scout = UnitAsset {
        base: AnimationSpriteAsset {
            image: base_img.clone(),
            layout: layouts.add(create_atlas(64, 64, 7, 1)),
            animation: animations.add(create_animation(base_img, 7, 1, AnimationRepeat::Loop)),
        },
        engine: AnimationSpriteAsset {
            image: engine_img.clone(),
            layout: layouts.add(create_atlas(64, 64, 8, 1)),
            animation: animations.add(create_animation(engine_img, 8, 1, AnimationRepeat::Loop)),
        },
        destruction: AnimationSpriteAsset {
            image: destruction_img.clone(),
            layout: layouts.add(create_atlas(64, 64, 9, 1)),
            animation: animations.add(create_animation(
                destruction_img,
                9,
                1,
                AnimationRepeat::Times(1),
            )),
        },
    };

    cmds.insert_resource(GameAssets {
        units: UnitAssets { biomantes_scout },

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

fn update(mut death_reader: MessageReader<DeathEvent>) {
    for event in death_reader.read() {
        println!("Entity {:?} died", event.entity);
        println!("Killer is {:?}", event.killer);

        // cmds.entity(event.entity).despawn();
    }
}

fn destruction_animation_system(mut cmds: Commands, mut events: MessageReader<AnimationEvent>) {
    for event in events.read() {
        if let AnimationEvent::AnimationEnd { entity, .. } = event {
            cmds.entity(*entity).despawn();
        }
    }
}

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
    mut player: Single<(Entity, &mut MoveTarget, &mut Transform), (With<Player>, Without<Dying>)>,
    mut death_writer: MessageWriter<DeathEvent>,
) {
    if btns.just_pressed(MouseButton::Right) {
        player.1.target = Some(cursor.world);
    }

    if btns.just_pressed(MouseButton::Middle) {
        let destruction = &game_assets.units.biomantes_scout.destruction;

        cmds.entity(player.0)
            .remove::<MoveTarget>()
            .remove::<Velocity>()
            .insert(destruction.sprite())
            .insert(destruction.ssanimation())
            .insert(Dying)
            .despawn_children();

        death_writer.write(DeathEvent {
            entity: player.0,
            killer: player.0,
        });
    }

    if btns.just_pressed(MouseButton::Left) {
        let projectile = &game_assets.projectiles.corrosion_wave;

        let direction = cursor.world - player.2.translation.truncate();

        if direction.length_squared() > 0.0001 {
            let mut transform = Transform::from_translation(player.2.translation);
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
    }
}

fn unit_ai_system(
    time: Res<Time>,
    player: Single<&Transform, (With<Player>, Without<Enemy>)>,
    mut query: Query<(&mut MoveTarget, &mut EnemyAI), (With<Enemy>, Without<Player>)>,
) {
    let player_pos = player.translation.truncate();

    for (mut target, mut ai) in &mut query {
        ai.timer.tick(time.delta());

        if !ai.timer.just_finished() {
            continue;
        }

        let offset = Vec2::new(
            fastrand::f32() * 500.0 - 250.0,
            fastrand::f32() * 500.0 - 250.0,
        );

        target.target = Some(player_pos + offset);
    }
}

fn camera_rect(camera: &Transform, bounds: &CameraBounds) -> (f32, f32, f32, f32) {
    let center = camera.translation.truncate();

    (
        center.x - bounds.half_size.x,
        center.x + bounds.half_size.x,
        center.y - bounds.half_size.y,
        center.y + bounds.half_size.y,
    )
}

fn unit_spawn_system(
    mut cmds: Commands,
    time: Res<Time>,
    mut spawn_timer: ResMut<SpawnTimer>,
    bounds: Res<CameraBounds>,
    camera: Single<&Transform, With<Camera>>,
    game_assets: Res<GameAssets>,
) {
    spawn_timer.0.tick(time.delta());

    if !spawn_timer.0.just_finished() {
        return;
    }

    let (left, right, bottom, top) = camera_rect(&camera, &bounds);

    let offset = 150.0;

    let position = match fastrand::usize(0..4) {
        0 => Vec2::new(
            fastrand::f32() * (right - left) + left,
            top + offset,
        ),

        1 => Vec2::new(
            fastrand::f32() * (right - left) + left,
            bottom - offset,
        ),

        2 => Vec2::new(
            left - offset,
            fastrand::f32() * (top - bottom) + bottom,
        ),

        _ => Vec2::new(
            right + offset,
            fastrand::f32() * (top - bottom) + bottom,
        ),
    };

    let scout = &game_assets.units.biomantes_scout;

    cmds.spawn((
        Enemy,
        EnemyAI {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        },
        Unit,
        MoveTarget::default(),
        Velocity::default(),
        MaxSpeed(150.0),
        Transform::from_translation(position.extend(0.0)),
        scout.base.sprite(),
        scout.base.ssanimation(),
    ));
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
