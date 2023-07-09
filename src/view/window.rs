use kiss3d::camera::ArcBall;
use kiss3d::light::Light;
use kiss3d::nalgebra::{Point3, Point2};
use kiss3d::text::Font;
use kiss3d::window::Window;

use super::virtualcube::VirtualCubeMeshes;

pub fn run() {
    let mut window = Window::new("cube");
    let mut scene = window.add_group();

    let cube = VirtualCubeMeshes::new(0.2, 0.02, 0.1);
    cube.add_meshes(&mut scene);
    window.set_light(Light::StickToCamera);

    let mut camera = ArcBall::new(Point3::new(0.5, 0.7, 1.0), Point3::default());
    camera.rebind_drag_button(None);

    let red = Point3::new(1.0, 0.0, 0.0);
    let center = Point2::new(0.5, 0.5);
    let font = Font::default();
    window.draw_text("text", &center, 10.0, &font, &red);
    while window.render_with_camera(&mut camera) {
        window.draw_text("text", &center, 10.0, &font, &red);
    }
}
