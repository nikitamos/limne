#[macro_export]
macro_rules! with {
  ($x:ident: $($field:ident = $val: expr), *) => {
      {
        let mut y = $x;
        $(y.$field = $val;)*
        y
      }
  };
}
