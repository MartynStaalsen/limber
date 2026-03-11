pub trait Block {
  fn execute(&mut self);
}