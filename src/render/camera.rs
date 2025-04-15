use cgmath::{
  Deg, EuclideanSpace, Matrix4, Point3, Quaternion, Rad, Rotation, Rotation3, Vector2, Vector3,
};

pub struct CameraController {
  look_at: Point3<f32>,
  right: Vector3<f32>,
  up: Vector3<f32>,
  r: f32,
}

impl Default for CameraController {
  fn default() -> Self {
    Self {
      look_at: Point3::origin(),
      right: Vector3::unit_x(),
      up: Vector3::unit_y(),
      r: 10.,
    }
  }
}

#[repr(transparent)]
pub struct Camera(Matrix4<f32>);
impl Camera {
  pub fn project_with(&self, proj: Projection) {
    todo!()
  }
}

struct Projection {
  fov: f32,
  aspect: f32,
}

impl CameraController {
  pub fn look_at(&mut self, point: Point3<f32>) -> &mut Self {
    self.look_at = point;
    self
  }
  pub fn handle_drag(&mut self, vec: egui::Vec2) -> &mut Self {
    if vec.length() == 0. {
      return self;
    }
    let up_rot = Quaternion::from_axis_angle(self.right, Rad(vec.y / self.r));
    let right_rot = Quaternion::from_axis_angle(self.up, Rad(vec.x / self.r));

    self.up = up_rot.rotate_vector(self.up);
    self.right = right_rot.rotate_vector(self.right);
    dbg!(self.right);
    dbg!(self.up);
    self
  }

  pub fn get_camera(&self) -> Matrix4<f32> {
    Matrix4::look_at_rh(
      self.look_at + self.r * self.right.cross(self.up),
      self.look_at,
      self.up,
    )
  }
}
