use std::{cell::RefCell, rc::Rc};

use kiss3d::{
    nalgebra::{Point3, Vector3},
    resource::Mesh,
    scene::SceneNode,
};
use palette::{rgb::Rgb, Hsv, IntoColor};

type Array3D<T, const I: usize, const J: usize, const K: usize> = [[[T; K]; J]; I];
type VirtualFaceMeshes = (/*front*/ Rc<RefCell<Mesh>>, /*back*/ Rc<RefCell<Mesh>>);
pub struct VirtualCubeMeshes(Array3D<VirtualFaceMeshes, /*face*/ 6, /*row*/ 3, /*column*/ 3>);

fn uvw_to_xyz(f: usize, p: Point3<f32>) -> Point3<f32> {
    match f {
        0 => Point3::new(p.z, p.x, p.y),
        1 => Point3::new(-p.y, -p.x, -p.z),
        2 => Point3::new(p.y, p.z, p.x),
        3 => Point3::new(-p.z, -p.y, -p.x),
        4 => Point3::new(p.x, p.y, p.z),
        5 => Point3::new(-p.x, -p.z, -p.y),
        _ => panic!(),
    }
}

fn square(p0: Point3<f32>, p1: Point3<f32>, p2: Point3<f32>, p3: Point3<f32>) -> Mesh {
    Mesh::new(
        vec![p0, p1, p2, p3],
        vec![Point3::new(0, 1, 2), Point3::new(0, 2, 3)],
        None,
        None,
        true,
    )
}

impl VirtualCubeMeshes {
    pub fn new(radius: f32, gap: f32, raise: f32) -> VirtualCubeMeshes {
        let step = (radius * 2.0 + gap) / 3.0;
        let width = (radius * 2.0 - gap * 2.0) / 3.0;
        VirtualCubeMeshes({
            core::array::from_fn(|f| {
                core::array::from_fn(|r| {
                    core::array::from_fn(|c| {
                        let u0 = -radius + (r as f32) * step;
                        let v0 = -radius + (c as f32) * step;
                        let u1 = u0 + width;
                        let v1 = v0 + width;
                        let front = square(
                            uvw_to_xyz(f, Point3::new(u0, v0, radius)),
                            uvw_to_xyz(f, Point3::new(u1, v0, radius)),
                            uvw_to_xyz(f, Point3::new(u1, v1, radius)),
                            uvw_to_xyz(f, Point3::new(u0, v1, radius)),
                        );
                        let back = square(
                            uvw_to_xyz(f, Point3::new(u0, v0, radius + raise)),
                            uvw_to_xyz(f, Point3::new(u0, v1, radius + raise)),
                            uvw_to_xyz(f, Point3::new(u1, v1, radius + raise)),
                            uvw_to_xyz(f, Point3::new(u1, v0, radius + raise)),
                        );
                        (Rc::new(RefCell::new(front)), Rc::new(RefCell::new(back)))
                    })
                })
            })
        })
    }

    pub fn add_meshes(&self, scene: &mut SceneNode) {
        self.0.iter().enumerate().for_each(|(f, m)| {
            m.iter().enumerate().for_each(|(_r, m)| {
                m.iter().enumerate().for_each(|(_c, (front, back))| {
                    let mut face_f = scene.add_mesh(Rc::clone(front), Vector3::new(1.0, 1.0, 1.0));
                    let mut face_b = scene.add_mesh(Rc::clone(back), Vector3::new(1.0, 1.0, 1.0));
                    let color: Rgb = Hsv::new(60.0 * (f as f32), 1.0, 1.0).into_color();
                    face_b.set_color(color.red, color.green, color.blue);
                    face_f.set_color(color.red, color.green, color.blue);
                })
            })
        });
    }
}
