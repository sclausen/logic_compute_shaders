use bevy::{
    math::{cubic_splines::CubicCurve, vec3},
    prelude::*,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

fn bezier_point(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let one_minus_t = 1.0 - t;
    one_minus_t.powi(3) * p0
        + 3.0 * one_minus_t.powi(2) * t * p1
        + 3.0 * one_minus_t * t.powi(2) * p2
        + t.powi(3) * p3
}

// Calculates the first derivative (tangent) of a cubic Bézier curve at a given t
fn bezier_tangent(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let derivative_p0 = 3.0 * (p1 - p0);
    let derivative_p1 = 3.0 * (p2 - p1);
    let derivative_p2 = 3.0 * (p3 - p2);

    bezier_point(derivative_p0, derivative_p1, derivative_p2, Vec3::ZERO, t).normalize()
}

fn bezier_normal(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32, up: Vec3) -> Vec3 {
    let tangent = bezier_tangent(p0, p1, p2, p3, t);
    let binormal = tangent.cross(up).normalize();
    binormal.cross(tangent).normalize()
}

fn generate_points_on_edge(position: Vec3, normal: Vec3, radius: f32, n: usize) -> Vec<Vec3> {
    let mut points = Vec::with_capacity(n);

    // Normalize the normal vector to ensure it's a unit vector
    let normal = normal.normalize();

    // Find an arbitrary vector that is not parallel to the normal
    let arbitrary_vector = if normal.x.abs() > 0.9 {
        Vec3::Y
    } else {
        Vec3::X
    };

    let u = arbitrary_vector.cross(normal).normalize();
    let v = normal.cross(u).normalize();

    for i in 0..n {
        let theta = 2.0 * std::f32::consts::PI * (i as f32) / (n as f32);
        let point_on_circle = position + radius * (u * theta.cos() + v * theta.sin());
        points.push(point_on_circle);
    }

    points
}

#[derive(Component)]
struct Curve(CubicCurve<Vec3>);

#[derive(Resource, Default)]
struct SplineConfig(Vec<Vec3>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PanOrbitCameraPlugin)
        .init_resource::<SplineConfig>()
        .add_systems(Startup, setup)
        .add_systems(Update, draw_curve)
        .run();
}

fn setup(mut commands: Commands, mut spline_config: ResMut<SplineConfig>) {
    let points = [[
        vec3(-1.0, -20.0, 0.0),
        vec3(3.0, 2.0, 0.0),
        vec3(5.0, 3.0, 0.0),
        vec3(9.0, 8.0, 0.0),
    ]];

    let [[p0, p1, p2, p3]] = points;

    *spline_config = SplineConfig(vec![p0, p1, p2, p3]);

    // Make a CubicCurve
    let bezier = CubicBezier::new(points).to_curve();
    let mut vertices: Vec<Vec3> = vec![];

    bezier.iter_positions(50).enumerate().for_each(|(i, pos)| {
        let tangent = bezier_tangent(
            spline_config.0[0],
            spline_config.0[1],
            spline_config.0[2],
            spline_config.0[3],
            i as f32 / 50.0,
        );
        vertices.extend(generate_points_on_edge(pos, tangent, 1.0, 8));
    });

    // Spawning a cube to experiment on
    commands.spawn((Curve(bezier),));

    // Some light to see something
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            ..default()
        },
        transform: Transform::from_xyz(8., 16., 8.),
        ..default()
    });

    // The camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0., 6., 12.).looking_at(Vec3::new(0., 3., 0.), Vec3::Y),
            ..default()
        },
        PanOrbitCamera::default(),
    ));
}

fn draw_curve(
    time: Res<Time>,
    mut query: Query<&Curve>,
    mut gizmos: Gizmos,
    spline_config: ResMut<SplineConfig>,
) {
    let t = (time.elapsed_seconds().sin() + 1.0) / 2.0;
    for cubic_curve in &mut query {
        let tangent = bezier_tangent(
            spline_config.0[0],
            spline_config.0[1],
            spline_config.0[2],
            spline_config.0[3],
            t,
        );
        gizmos.linestrip(cubic_curve.0.iter_positions(50), Color::WHITE);
        let start = cubic_curve.0.position(t);
        gizmos.arrow(start, start + tangent, Color::RED);

        if let Ok(dir) = Direction3d::new(tangent) {
            gizmos.circle(start, dir, 1.0, Color::RED);
        }

        cubic_curve
            .0
            .iter_positions(50)
            .enumerate()
            .for_each(|(i, pos)| {
                let tangent = bezier_tangent(
                    spline_config.0[0],
                    spline_config.0[1],
                    spline_config.0[2],
                    spline_config.0[3],
                    i as f32 / 50.0,
                );
                if let Ok(tangent) = Direction3d::new(tangent) {
                    gizmos.circle(pos, tangent, 1.0, Color::GREEN);
                }
            });
    }
}
