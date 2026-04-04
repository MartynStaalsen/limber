use crate::objs::signal::{Value, SignalType};
use std::marker::PhantomData;


#[derive(Clone)]
pub struct SignalHandle<T> {
    index: usize,
    phantom: PhantomData<T>,
}

impl<T> SignalHandle<T> {
    pub(crate) fn new(index: usize) -> Self{   
        SignalHandle {index, phantom: PhantomData }
    }
}

#[derive(Clone)]
pub struct SignalReader<T>(SignalHandle<T>);

pub struct SignalWriter<T>(SignalHandle<T>); // no clone, single owner

impl<T> SignalReader<T> {
    pub(crate) fn new(index: usize) -> Self { SignalReader(SignalHandle::new(index)) }
}

impl<T> SignalWriter<T> {
    pub(crate) fn new(index: usize) -> Self{ SignalWriter(SignalHandle::new(index)) }
}


pub struct SignalBus {
    values: Vec<Option<Value>>,
}

impl SignalBus {
    pub fn new() -> Self {
        SignalBus { values: Vec::new() }
    }

    pub fn allocate<T: SignalType>(&mut self) -> (SignalReader<T>, SignalWriter<T>) {
        let idx = self.values.len();
        self.values.push(None);
        (SignalReader::new(idx), SignalWriter::new(idx))
    }

    pub fn read<T: SignalType>(&self, reader: &SignalReader<T>) -> T {
        match self.values[reader.0.index].clone() {
            Some(v) => T::try_from(v).unwrap_or_else(|_| unreachable!("type guaranteed by SignalReader<T>")),
            None => panic!("signal read before write")
        }
    }

    pub fn write<T: SignalType>(&mut self, writer: &SignalWriter<T>, val: T) {
        self.values[writer.0.index] = Some(val.into());
    }
}
