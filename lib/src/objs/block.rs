use crate::objs::signal_bus::SignalBus;

pub trait Block {
  fn execute(&mut self, bus: &mut SignalBus);
}