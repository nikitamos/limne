#![feature(option_zip)]
#![feature(duration_millis_float)]
#![feature(associated_type_defaults)]
#![feature(more_float_constants)]
#![feature(more_qualified_paths)]
#[macro_export]
macro_rules! with {
  ($x:ident: $($($fields:ident).* = $val: expr), *) => {
      {
        let mut y = $x;
        $(y$(.$fields)* = $val;)*
        y
      }
  };
  ($x:expr => $($($fields:ident).* = $val: expr), *) => {
      {
        let mut y = $x;
        // TODO: Reuse arm #0
        $(y$(.$fields)* = $val;)*
        y
      }
  };
}
pub mod render;
pub mod solvers;
