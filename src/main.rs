use bevy::{
    camera::ScalingMode,
    prelude::*,
    window::{EnabledButtons, WindowResolution},
};

use bevy_rapier2d::prelude::*;
use bevy_spritesheet_animation::prelude::*;

const U_GAME_WIDTH: u32 = 1920;
const U_GAME_HEIGHT: u32 = 1080;

const F_GAME_WIDTH: f32 = 1920.0;
const F_GAME_HEIGHT: f32 = 1080.0;

const MAX_DIST_TO_PLAYER: f32 = 2400.0;
const MAX_UNITS: u32 = 10;

const ATTACK_RANGE: f32 = 400.0;
const ATTACK_PER_SECOND: f32 = 1.0;
const ROTATION_SPEED: f32 = 10.0;
const ATTACK_ANGLE_THRESHOLD: f32 = 5.0_f32.to_radians();
const TARGET_SELECTION_RADIUS: f32 = 32.0;

const PROJECTILE_SPEED: f32 = 400.0;
const PROJECTILE_LIFETIME: f32 = 2.0;

const CLICK_DELAY: f32 = 0.125;
const SPAWN_TIMER: f32 = 2.0;
const ENGINE_HIDE_TIMER: f32 = 0.8;

const MOVE_MARKER_LIFETIME: f32 = 0.4;
const MOVE_MARKER_START_RADIUS: f32 = 1.0;
const MOVE_MARKER_END_RADIUS: f32 = 4.0;
const MOVE_MARKER_THICKNESS: f32 = 0.5;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(U_GAME_WIDTH, U_GAME_HEIGHT),
                    resizable: true,
                    enabled_buttons: EnabledButtons {
                        maximize: true,
                        minimize: true,
                        close: true,
                    },
                    mode: bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }), // .set(ImagePlugin::default_nearest())
            SpritesheetAnimationPlugin,
            RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),
        ))
        .add_message::<DeathEvent>()
        .add_systems(Startup, (setup, spawn_player_unit).chain())
        .add_systems(
            Update,
            (
                update,
                cursor_moved_system,
                camera_input_system,
                player_input_system,
                (unit_rotation_system, unit_movement_system).chain(),
                // velocity_system,
                unit_ai_system,
                unit_spawn_system,
                engine_system.after(unit_movement_system),
                // projectile_movement_system,
                projectile_life_system,
                destruction_animation_system,
                projectile_vanish_system,
                // unit_state_system,
                unit_attack_system,
                input_system,
                click_delay_system,
                move_marker_system,
            ),
        )
        .run();
}

#[derive(Resource)]
struct GlobalVars {
    unit_count: u32,
    click_repeat: bool,
}

#[derive(Resource)]
struct ClickDelay(Timer);

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
struct MoveMarker {
    timer: Timer,
    material: Handle<ColorMaterial>,
}

#[derive(Component)]
struct Player;

#[derive(Component, Clone, Copy, Default)]
enum UnitCommand {
    #[default]
    Idle,
    MoveTo(Vec2),
    Attack(Entity),
}

#[derive(Component)]
struct MaxSpeed(f32);

#[derive(Component)]
struct RotationSpeed(f32);

#[derive(Component)]
struct AttackStats {
    range: f32,
    attack_per_second: f32,
}

#[derive(Component, Default)]
struct AttackCooldown {
    remaining: f32,
}

#[derive(Component)]
struct Unit;

#[derive(Component)]
struct Dying;

#[derive(Component)]
struct Engine {
    entity: Entity,
    hide_timer: Timer,
}

#[derive(Resource)]
struct CameraBounds {
    half_size: Vec2,
}

#[derive(Component)]
struct AI;

#[derive(Resource)]
struct SpawnTimer(Timer);

#[derive(Component)]
struct Projectile {
    life_time: Timer,
}

#[derive(Component)]
struct Vanish {
    base_life_time: f32,
}

#[derive(Message)]
struct DeathEvent {
    entity: Entity,
    killer: Entity,
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

    cmds.insert_resource(SpawnTimer(Timer::from_seconds(
        SPAWN_TIMER,
        TimerMode::Repeating,
    )));

    cmds.insert_resource(ClickDelay(Timer::from_seconds(
        CLICK_DELAY,
        TimerMode::Repeating,
    )));

    cmds.insert_resource(GlobalVars {
        unit_count: 0,
        click_repeat: true,
    });

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

fn click_delay_system(
    time: Res<Time>,
    mut click_timer: ResMut<ClickDelay>,
    mut globals: ResMut<GlobalVars>,
) {
    click_timer.0.tick(time.delta());

    if click_timer.0.just_finished() {
        globals.click_repeat = true;
    }
}

fn spawn_player_unit(mut cmds: Commands, game_assets: Res<GameAssets>) {
    let transform = Transform::from_xyz(0.0, 0.0, 0.0);

    let player_unit = &game_assets.units.biomantes_scout;

    let entt = cmds
        .spawn((
            Player,
            Unit,
            UnitCommand::default(),
            MaxSpeed(200.0),
            RotationSpeed(ROTATION_SPEED),
            AttackStats {
                attack_per_second: ATTACK_PER_SECOND,
                range: ATTACK_RANGE,
            },
            AttackCooldown::default(),
            RigidBody::KinematicVelocityBased,
            Collider::ball(24.0),
            Velocity::default(),
            transform,
            player_unit.base.sprite(),
            player_unit.base.ssanimation(),
        ))
        .id();

    cmds.spawn((
        Engine {
            entity: entt,
            hide_timer: Timer::from_seconds(ENGINE_HIDE_TIMER, TimerMode::Once),
        },
        Visibility::Visible,
        transform,
        player_unit.engine.sprite(),
        player_unit.engine.ssanimation(),
    ))
    .set_parent_in_place(entt);
}

fn update(mut globals: ResMut<GlobalVars>, mut death_reader: MessageReader<DeathEvent>) {
    for event in death_reader.read() {
        println!("Entity {:?} died", event.entity);
        println!("Killer is {:?}", event.killer);

        globals.unit_count = globals.unit_count.saturating_sub(1);
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

fn input_system(kb: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if kb.just_pressed(KeyCode::KeyQ) {
        exit.write(AppExit::Success);
    }
}

fn player_input_system(
    mut cmds: Commands,
    game_assets: Res<GameAssets>,
    btns: Res<ButtonInput<MouseButton>>,
    kb: Res<ButtonInput<KeyCode>>,
    cursor: Res<CursorCoords>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut globals: ResMut<GlobalVars>,
    mut player: Single<(Entity, &mut UnitCommand), (With<Player>, Without<Dying>)>,
    units: Query<(Entity, &GlobalTransform), (With<AI>, Without<Dying>)>,
    mut death_writer: MessageWriter<DeathEvent>,
) {
    if (btns.pressed(MouseButton::Right) || kb.pressed(KeyCode::KeyE)) && globals.click_repeat {
        let mesh = meshes.add(Annulus::new(
            MOVE_MARKER_START_RADIUS - MOVE_MARKER_THICKNESS * 0.5,
            MOVE_MARKER_START_RADIUS + MOVE_MARKER_THICKNESS * 0.5,
        ));

        let material = materials.add(ColorMaterial {
            color: Color::srgba(0.0, 1.0, 0.0, 1.0),
            ..default()
        });

        cmds.spawn((
            MoveMarker {
                timer: Timer::from_seconds(MOVE_MARKER_LIFETIME, TimerMode::Once),
                material: material.clone(),
            },
            Mesh2d(mesh),
            MeshMaterial2d(material),
            Transform::from_translation(cursor.world.extend(1.0)),
        ));

        globals.click_repeat = false;

        let clicked_target = units
            .iter()
            .filter_map(|(entity, transform)| {
                let target_position = transform.translation().truncate();
                let distance = target_position.distance(cursor.world);

                (distance <= TARGET_SELECTION_RADIUS).then_some((entity, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(entity, _)| entity);

        match clicked_target {
            Some(target) => {
                *player.1 = UnitCommand::Attack(target);
            }

            None => {
                *player.1 = UnitCommand::MoveTo(cursor.world);
            }
        }
    }

    if btns.just_pressed(MouseButton::Middle) {
        let destruction = &game_assets.units.biomantes_scout.destruction;

        cmds.entity(player.0)
            .insert(destruction.sprite())
            .insert(destruction.ssanimation())
            .insert(Dying)
            .remove::<Collider>()
            .remove::<RigidBody>()
            .remove::<Velocity>()
            .despawn_children();

        death_writer.write(DeathEvent {
            entity: player.0,
            killer: player.0,
        });
    }
}

fn move_marker_system(
    mut cmds: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query: Query<(Entity, &mut MoveMarker, &mut Transform)>,
) {
    for (entity, mut marker, mut transform) in &mut query {
        marker.timer.tick(time.delta());

        let progress = marker.timer.fraction();

        let radius = MOVE_MARKER_START_RADIUS.lerp(MOVE_MARKER_END_RADIUS, progress);

        let scale = radius / MOVE_MARKER_START_RADIUS;

        transform.scale = Vec3::splat(scale);

        if let Some(material) = materials.get_mut(&marker.material) {
            material.color = Color::srgba(
                0.0,
                1.0,
                0.0,
                1.0 - progress,
            );
        }

        if marker.timer.just_finished() {
            cmds.entity(entity).despawn();
        }
    }
}

fn unit_ai_system(
    player: Single<&Transform, With<Player>>,
    mut query: Query<(&Transform, &mut UnitCommand), (With<AI>, Without<Dying>)>,
) {
    let player_position = player.translation.truncate();

    for (transform, mut command) in &mut query {
        if !matches!(*command, UnitCommand::Idle) {
            continue;
        }

        let enemy_position = transform.translation.truncate();

        if enemy_position.distance(player_position) < MAX_DIST_TO_PLAYER {
            continue;
        }

        if let Some(target) = ai_calculate_target(player_position, enemy_position) {
            *command = UnitCommand::MoveTo(target);
        }
    }
}

fn unit_rotation_system(
    mut query: Query<(&mut Transform, &UnitCommand, &RotationSpeed), (With<Unit>, Without<Dying>)>,
    targets: Query<&GlobalTransform, (With<Unit>, Without<Dying>)>,
    time: Res<Time>,
) {
    for (mut transform, command, rotation_speed) in &mut query {
        let target_position = match *command {
            UnitCommand::Idle => None,

            UnitCommand::MoveTo(position) => Some(position),

            UnitCommand::Attack(target) => {
                targets.get(target).ok().map(|t| t.translation().truncate())
            }
        };

        let Some(target_position) = target_position else {
            continue;
        };

        rotate_toward(
            &mut transform,
            target_position,
            rotation_speed.0,
            time.delta_secs(),
        );
    }
}

fn unit_movement_system(
    mut query: Query<
        (&mut UnitCommand, &Transform, &MaxSpeed, &mut Velocity),
        (With<Unit>, Without<Dying>),
    >,
    targets: Query<(&GlobalTransform, Option<&Dying>), (With<Unit>, Without<Dying>)>,
) {
    for (mut command, transform, speed, mut velocity) in &mut query {
        let target_position = match *command {
            UnitCommand::Idle => {
                velocity.linear = Vec2::ZERO;
                continue;
            }

            UnitCommand::MoveTo(destination) => Some(destination),

            UnitCommand::Attack(target) => {
                let Ok((target_transform, _)) = targets.get(target) else {
                    *command = UnitCommand::Idle;
                    velocity.linear = Vec2::ZERO;
                    continue;
                };

                let target_position = target_transform.translation().truncate();
                let direction = target_position - transform.translation.truncate();

                // Stop once the target is inside attack range.
                if direction.length_squared() <= ATTACK_RANGE * ATTACK_RANGE {
                    velocity.linear = Vec2::ZERO;
                    continue;
                }

                Some(target_position)
            }
        };

        let Some(target_position) = target_position else {
            velocity.linear = Vec2::ZERO;
            continue;
        };

        let direction = target_position - transform.translation.truncate();

        if matches!(*command, UnitCommand::MoveTo(_)) && direction.length_squared() <= 4.0 {
            *command = UnitCommand::Idle;
            velocity.linear = Vec2::ZERO;
            continue;
        }

        if !is_facing_target(transform, target_position) {
            velocity.linear = Vec2::ZERO;
            continue;
        }

        if direction.length_squared() <= 0.0001 {
            velocity.linear = Vec2::ZERO;
        } else {
            velocity.linear = direction.normalize() * speed.0;
        }
    }
}

fn unit_attack_system(
    mut cmds: Commands,
    mut attackers: Query<
        (
            Entity,
            &Transform,
            &mut UnitCommand,
            &AttackStats,
            &mut AttackCooldown,
        ),
        (With<Unit>, Without<Dying>),
    >,
    targets: Query<(&GlobalTransform, Option<&Dying>), With<Unit>>,
    game_assets: Res<GameAssets>,
    time: Res<Time>,
) {
    for (attacker, attacker_transform, mut command, stats, mut cooldown) in &mut attackers {
        cooldown.remaining = (cooldown.remaining - time.delta_secs()).max(0.0);

        let UnitCommand::Attack(target) = *command else {
            continue;
        };

        let Ok((target_transform, dying)) = targets.get(target) else {
            *command = UnitCommand::Idle;
            continue;
        };

        if dying.is_some() {
            *command = UnitCommand::Idle;
            continue;
        }

        let attacker_position = attacker_transform.translation.truncate();
        let target_position = target_transform.translation().truncate();
        let distance = attacker_position.distance(target_position);

        if distance > stats.range {
            continue;
        }

        if !is_facing_target(attacker_transform, target_position) {
            continue;
        }

        if cooldown.remaining > 0.0 {
            continue;
        }

        let direction = target_position - attacker_position;

        if direction.length_squared() <= 0.0001 {
            continue;
        }

        spawn_projectile(
            &mut cmds,
            &game_assets,
            attacker_transform.translation,
            direction.normalize(),
        );

        cooldown.remaining = 1.0 / stats.attack_per_second;

        println!("Entity {:?} shoots at {:?}", attacker, target);
    }
}

fn spawn_projectile(
    cmds: &mut Commands,
    game_assets: &GameAssets,
    position: Vec3,
    direction: Vec2,
) {
    let projectile = &game_assets.projectiles.corrosion_wave;

    let mut transform = Transform::from_translation(position);
    let angle = direction.y.atan2(direction.x);

    transform.rotation = Quat::from_rotation_z(angle - std::f32::consts::FRAC_PI_2);

    cmds.spawn((
        Projectile {
            life_time: Timer::from_seconds(PROJECTILE_LIFETIME, TimerMode::Once),
        },
        Vanish {
            base_life_time: PROJECTILE_LIFETIME,
        },
        RigidBody::KinematicVelocityBased,
        Collider::ball(16.0),
        Velocity {
            linear: direction.normalize() * PROJECTILE_SPEED,
            angular: 0.0,
        },
        transform,
        projectile.sprite(),
        projectile.ssanimation(),
    ));
}

fn unit_spawn_system(
    mut cmds: Commands,
    time: Res<Time>,
    mut globals: ResMut<GlobalVars>,
    mut spawn_timer: ResMut<SpawnTimer>,
    player: Single<&Transform, (With<Player>, Without<AI>)>,
    bounds: Res<CameraBounds>,
    camera: Single<&Transform, With<Camera>>,
    game_assets: Res<GameAssets>,
) {
    spawn_timer.0.tick(time.delta());

    if !spawn_timer.0.just_finished() {
        return;
    }

    if globals.unit_count >= MAX_UNITS {
        return;
    }

    let (left, right, bottom, top) = camera_rect(&camera, &bounds);
    let offset = 150.0;

    let position = match fastrand::usize(0..4) {
        0 => Vec2::new(fastrand::f32() * (right - left) + left, top + offset),

        1 => Vec2::new(fastrand::f32() * (right - left) + left, bottom - offset),

        2 => Vec2::new(left - offset, fastrand::f32() * (top - bottom) + bottom),

        _ => Vec2::new(right + offset, fastrand::f32() * (top - bottom) + bottom),
    };

    let scout = &game_assets.units.biomantes_scout;
    let transform = Transform::from_translation(position.extend(0.0));

    let command = ai_calculate_target(player.translation.truncate(), position)
        .map(UnitCommand::MoveTo)
        .unwrap_or(UnitCommand::Idle);

    let entity = cmds
        .spawn((
            AI,
            Unit,
            command,
            MaxSpeed(200.0),
            RotationSpeed(ROTATION_SPEED),
            AttackStats {
                range: ATTACK_RANGE,
                attack_per_second: ATTACK_PER_SECOND,
            },
            AttackCooldown::default(),
            RigidBody::KinematicVelocityBased,
            Collider::ball(24.0),
            Velocity::default(),
            transform,
            scout.base.sprite(),
            scout.base.ssanimation(),
        ))
        .id();

    cmds.spawn((
        Engine {
            entity,
            hide_timer: Timer::from_seconds(ENGINE_HIDE_TIMER, TimerMode::Once),
        },
        Visibility::Hidden,
        transform,
        scout.engine.sprite(),
        scout.engine.ssanimation(),
    ))
    .set_parent_in_place(entity);

    globals.unit_count += 1;
}

fn projectile_life_system(
    mut cmds: Commands,
    mut query: Query<(Entity, &mut Projectile)>,
    time: Res<Time>,
) {
    for (entity, mut projectile) in &mut query {
        projectile.life_time.tick(time.delta());

        if projectile.life_time.is_finished() {
            cmds.entity(entity).despawn();
        }
    }
}

fn projectile_vanish_system(mut query: Query<(&mut Sprite, &mut Projectile, &mut Vanish)>) {
    for (mut sprite, projectile, vanish) in &mut query {
        sprite.color.set_alpha(
            (projectile.life_time.remaining_secs() / vanish.base_life_time).clamp(0.0, 1.0),
        );
    }
}

fn engine_system(
    time: Res<Time>,
    mut query: Query<(&mut Visibility, &mut Engine)>,
    velocities: Query<&Velocity, With<Unit>>,
) {
    for (mut visibility, mut engine) in &mut query {
        let Ok(velocity) = velocities.get(engine.entity) else {
            continue;
        };

        let is_moving = velocity.linear.length_squared() > 0.0001;

        if is_moving {
            engine.hide_timer.reset();
            *visibility = Visibility::Visible;
        } else {
            engine.hide_timer.tick(time.delta());

            if engine.hide_timer.just_finished() {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

fn ai_calculate_target(player_position: Vec2, position: Vec2) -> Option<Vec2> {
    let to_player = (player_position - position).normalize();
    let angle = (fastrand::f32() - 0.5) * std::f32::consts::FRAC_PI_2;
    let direction = Mat2::from_angle(angle) * to_player;
    return Some(position + direction * 10000.0);
}

fn rotate_toward(
    transform: &mut Transform,
    target_position: Vec2,
    rotation_speed: f32,
    delta_secs: f32,
) {
    let position = transform.translation.truncate();
    let direction = target_position - position;

    if direction.length_squared() <= 0.0001 {
        return;
    }

    let target_angle = direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2;

    let (_, _, current_angle) = transform.rotation.to_euler(EulerRot::XYZ);

    let angle_difference = shortest_angle_difference(target_angle, current_angle);

    let max_rotation = rotation_speed * delta_secs;
    let rotation = angle_difference.clamp(-max_rotation, max_rotation);

    transform.rotate_z(rotation);
}

fn is_facing_target(transform: &Transform, target_position: Vec2) -> bool {
    let position = transform.translation.truncate();
    let direction = target_position - position;

    if direction.length_squared() <= 0.0001 {
        return true;
    }

    let target_angle = direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2;

    let (_, _, current_angle) = transform.rotation.to_euler(EulerRot::XYZ);

    shortest_angle_difference(target_angle, current_angle).abs() <= ATTACK_ANGLE_THRESHOLD
}

fn shortest_angle_difference(target: f32, current: f32) -> f32 {
    let mut difference = target - current;

    while difference > std::f32::consts::PI {
        difference -= std::f32::consts::TAU;
    }

    while difference < -std::f32::consts::PI {
        difference += std::f32::consts::TAU;
    }

    difference
}

fn camera_rect(camera: &Transform, bounds: &CameraBounds) -> (f32, f32, f32, f32) {
    let center = camera.translation.truncate();

    return (
        center.x - bounds.half_size.x,
        center.x + bounds.half_size.x,
        center.y - bounds.half_size.y,
        center.y + bounds.half_size.y,
    );
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
