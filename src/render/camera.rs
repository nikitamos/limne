use cgmath::{
  EuclideanSpace, InnerSpace, Matrix4, Point3, Quaternion, Rad, Rotation, Rotation3, Vector2,
  Vector3,
};

pub struct OrbitCameraController {
  center: Point3<f32>,
  right: Vector3<f32>,
  up: Vector3<f32>,
  r: f32,
}

impl Default for OrbitCameraController {
  fn default() -> Self {
    Self {
      center: Point3::origin(),
      right: Vector3::unit_x(),
      up: Vector3::unit_y(),
      r: 8.,
    }
  }
}

impl OrbitCameraController {
  pub fn look_at(&mut self, point: Point3<f32>) -> &mut Self {
    self.center = point;
    self
  }
  pub fn rotate_radians(&mut self, vec: egui::Vec2) -> &mut Self {
    if vec.length() == 0. {
      return self;
    }
    let up_rot = Quaternion::from_axis_angle(self.right, Rad(vec.y));
    let right_rot = Quaternion::from_axis_angle(self.up, Rad(vec.x));

    self.up = up_rot.rotate_vector(self.up);
    self.right = right_rot.rotate_vector(self.right);
    self
  }

  #[must_use]
  pub fn get_center(&self) -> Point3<f32> {
    self.center
  }
  pub fn get_radius(&self) -> f32 {
    self.r
  }
  #[must_use]
  pub fn get_pos(&self) -> Point3<f32> {
    self.center + self.right.cross(self.up) * self.r
  }
  pub fn reset(&mut self) -> &mut Self {
    *self = Self::default();
    self
  }

  pub fn move_center_global(&mut self, delta: Vector3<f32>) -> &mut Self {
    self.center += delta;
    self
  }

  /// Moves center in local coordinates
  /// X axis is facing 'up',
  /// Y axis is facing 'right'
  pub fn move_center_local(&mut self, delta: Vector2<f32>) -> &mut Self {
    self.center += self.up * delta.x + self.right * delta.y;
    self
  }

  pub fn move_radius(&mut self, dr: f32) -> &mut Self {
    self.r += dr;
    self
  }

  pub fn forward(&mut self, delta: f32) -> &mut Self {
    self.move_center_global(self.up.cross(self.right) * delta)
  }

  #[must_use]
  pub fn get_camera(&self) -> Matrix4<f32> {
    #[cfg(debug_assertions)]
    if (self.right.cross(self.up).magnitude2() - 1.0).abs() > 0.01 {
      dbg!(self.right.cross(self.up));
      log::error!(
        "Cross product is not normalized, right={}, up={}, angle={}rad",
        self.right.magnitude(),
        self.up.magnitude(),
        self.up.angle(self.right).0
      );
      unsafe {
        let this = std::ptr::from_ref(self).cast_mut();
        (*this).right = self.right.normalize();
        (*this).up = self.up.normalize();
      }
    }
    Matrix4::look_at_rh(
      self.center + self.r * self.right.cross(self.up),
      self.center,
      self.up,
    )
  }
}
