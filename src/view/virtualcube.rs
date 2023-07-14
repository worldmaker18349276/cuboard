#![allow(dead_code)]

use std::{cell::RefCell, f32::consts::PI, ops::{Mul, Neg}, rc::Rc};

use kiss3d::{
    camera::ArcBall,
    light::Light,
    nalgebra::{Point3, Quaternion, UnitQuaternion, Vector3},
    resource::Mesh,
    scene::SceneNode,
    window::Window,
};
use palette::{rgb::Rgb, Hsv, IntoColor};

type Array3D<T, const I: usize, const J: usize, const K: usize> = [[[T; K]; J]; I];
type VirtualFaceMeshes = (
    /*front*/ Rc<RefCell<Mesh>>,
    /*back*/ Rc<RefCell<Mesh>>,
);
type VirtualCubeMeshes =
    Array3D<VirtualFaceMeshes, /*face*/ 6, /*row*/ 3, /*column*/ 3>;
type VirtualFaceNodes = (/*front*/ SceneNode, /*back*/ SceneNode);
type VirtualCubeNodes =
    Array3D<VirtualFaceNodes, /*face*/ 6, /*row*/ 3, /*column*/ 3>;

fn uvw_to_xyz(f: usize, u: f32, v: f32, w: f32) -> Point3<f32> {
    match f {
        0 => Point3::new(w, u, v),
        1 => Point3::new(-v, -u, -w),
        2 => Point3::new(v, w, u),
        3 => Point3::new(-w, -v, -u),
        4 => Point3::new(u, v, w),
        5 => Point3::new(-u, -w, -v),
        _ => panic!(),
    }
}

const CENTERS: [Vector3<f32>; 6] = [
    Vector3::new(1.0, 0.0, 0.0),
    Vector3::new(0.0, 0.0, -1.0),
    Vector3::new(0.0, 1.0, 0.0),
    Vector3::new(-1.0, 0.0, 0.0),
    Vector3::new(0.0, 0.0, 1.0),
    Vector3::new(0.0, -1.0, 0.0),
];

fn square(p0: Point3<f32>, p1: Point3<f32>, p2: Point3<f32>, p3: Point3<f32>) -> Mesh {
    Mesh::new(
        vec![p0, p1, p2, p3],
        vec![Point3::new(0, 1, 2), Point3::new(0, 2, 3)],
        None,
        None,
        true,
    )
}

pub fn make_cube(radius: f32, gap: f32, raise: f32) -> VirtualCubeMeshes {
    let step = (radius * 2.0 + gap) / 3.0;
    let width = (radius * 2.0 - gap * 2.0) / 3.0;
    core::array::from_fn(|f| {
        core::array::from_fn(|r| {
            core::array::from_fn(|c| {
                let u0 = -radius + (r as f32) * step;
                let v0 = -radius + (c as f32) * step;
                let u1 = u0 + width;
                let v1 = v0 + width;
                let front = square(
                    uvw_to_xyz(f, u0, v0, radius),
                    uvw_to_xyz(f, u1, v0, radius),
                    uvw_to_xyz(f, u1, v1, radius),
                    uvw_to_xyz(f, u0, v1, radius),
                );
                let back = square(
                    uvw_to_xyz(f, u0, v0, radius + raise),
                    uvw_to_xyz(f, u0, v1, radius + raise),
                    uvw_to_xyz(f, u1, v1, radius + raise),
                    uvw_to_xyz(f, u1, v0, radius + raise),
                );
                (Rc::new(RefCell::new(front)), Rc::new(RefCell::new(back)))
            })
        })
    })
}

pub fn add_meshes(meshes: &VirtualCubeMeshes, scene: &mut SceneNode) -> VirtualCubeNodes {
    core::array::from_fn(|f| {
        core::array::from_fn(|r| {
            core::array::from_fn(|c| {
                let (front, back) = &meshes[f][r][c];
                let face_f = scene.add_mesh(Rc::clone(front), Vector3::new(1.0, 1.0, 1.0));
                let mut face_b = scene.add_mesh(Rc::clone(back), Vector3::new(1.0, 1.0, 1.0));
                face_b.set_visible(false);
                (face_f, face_b)
            })
        })
    })
}

#[allow(clippy::needless_range_loop)]
pub fn set_backface_visible(nodes: &mut VirtualCubeNodes, visible: bool) {
    for f in 0..6 {
        for r in 0..3 {
            for c in 0..3 {
                let (_face_f, face_b) = &mut nodes[f][r][c];
                face_b.set_visible(visible);
            }
        }
    }
}

pub struct VirtualCuboard {
    pub window: Window,
    pub node: SceneNode,
    pub components: VirtualCubeNodes,
    pub camera: ArcBall,
}

impl VirtualCuboard {
    const INIT_EYE: Vector3<f32> = Vector3::new(-1.0, 1.0, -1.0);

    pub fn new() -> Self {
        let mut window = Window::new("cube");
        let mut node = window.add_group();
        let mut components = add_meshes(&make_cube(0.2, 0.02, 0.1), &mut node);
        set_colors_gan(&mut components);
        let eye = Point3::new(Self::INIT_EYE.x, Self::INIT_EYE.y, Self::INIT_EYE.z);
        let camera = ArcBall::new(eye, Point3::default());
        VirtualCuboard {
            window,
            node,
            components,
            camera,
        }
    }

    pub fn render_loop<F: FnMut(&mut Self)>(&mut self, mut f: F) {
        self.window.set_light(Light::StickToCamera);
        self.camera.rebind_drag_button(None);

        while self.window.render_with_camera(&mut self.camera) {
            f(self)
        }
    }

    pub fn set_orientation(&mut self, orientation: UnitQuaternion<f32>) {
        self.node.set_local_rotation(orientation);
    }
}

// set colors by gancube
pub fn set_colors_gan(nodes: &mut VirtualCubeNodes) {
    let colors: [Rgb; 6] = [
        Hsv::new(240.0, 1.0, 1.0).into_color(),
        Hsv::new(300.0, 1.0, 1.0).into_color(),
        Hsv::new(000.0, 0.0, 1.0).into_color(),
        Hsv::new(120.0, 1.0, 1.0).into_color(),
        Hsv::new(000.0, 1.0, 1.0).into_color(),
        Hsv::new(060.0, 1.0, 1.0).into_color(),
    ];
    for f in 0..6 {
        for r in 0..3 {
            for c in 0..3 {
                let (face_f, face_b) = &mut nodes[f][r][c];
                let color = colors[f];
                face_b.set_color(color.red, color.green, color.blue);
                face_f.set_color(color.red, color.green, color.blue);
            }
        }
    }
}

// set colors by hue colormap
pub fn set_colors_hue(nodes: &mut VirtualCubeNodes, hue_offsets: [f32; 6]) {
    for f in 0..6 {
        for r in 0..3 {
            for c in 0..3 {
                let (face_f, face_b) = &mut nodes[f][r][c];
                let hue = (60.0 * (f as f32) + hue_offsets[f]).rem_euclid(360.0);
                let color: Rgb = Hsv::new(hue, 1.0, 1.0).into_color();
                face_b.set_color(color.red, color.green, color.blue);
                face_f.set_color(color.red, color.green, color.blue);
            }
        }
    }
}

// set colors by global orientation
pub fn set_colors_ori(nodes: &mut VirtualCubeNodes, orientation: UnitQuaternion<f32>) {
    fn half_angle(q: UnitQuaternion<f32>) -> f32 {
        (q.i.powi(2) + q.j.powi(2) + q.k.powi(2)).sqrt().atan2(q.w)
    }

    let angle = half_angle(orientation).mul(180.0 / PI).rem_euclid(360.0);
    set_colors_hue(nodes, [angle; 6]);
}

// set colors by spin angle to the eye
pub fn set_colors_spin(nodes: &mut VirtualCubeNodes, eye: Point3<f32>, orientation: UnitQuaternion<f32>) {
    fn rotate_to(v1: Vector3<f32>, v2: Vector3<f32>) -> UnitQuaternion<f32> {
        let xyz = v1.cross(&v2);
        let w = (v1.norm_squared() * v2.norm_squared()).sqrt() + v1.dot(&v2);
        UnitQuaternion::new_normalize(Quaternion::new(w, xyz.x, xyz.y, xyz.z))
    }

    fn spin_angle(q: UnitQuaternion<f32>, v: Vector3<f32>) -> f32 {
        Vector3::new(q.i, q.j, q.k).dot(&v.normalize()).neg().atan2(q.w)
    }

    let eye = Vector3::new(eye.x, eye.y, eye.z).normalize();
    let angles = core::array::from_fn(|f| {
        spin_angle(orientation * rotate_to(eye, CENTERS[f]), eye)
            .mul(180.0 / PI)
            .rem_euclid(360.0)
    });
    set_colors_hue(nodes, angles);
}
