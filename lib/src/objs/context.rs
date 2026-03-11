use crate::objs::signal_bus::SignalBus;
use crate::objs::block::Block;

pub struct Context {
  bus: SignalBus,
  blocks: Vec<Box<dyn Block>>
}

impl Context {
  pub fn run_cycle(&mut self){
    for block in &mut self.blocks {
      block.execute()
    }
  }
}