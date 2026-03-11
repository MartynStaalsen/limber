pub enum Value {
    Bool(bool),
    Int(i32),
    Float(f32)
}

pub trait SignalType: Into<Value> { // + TryFrom<Value> {
}

impl From<bool> for Value { // implements Into for SignalType
    fn from(val: bool) -> Value {
        Value::Bool(val)
    }
}
impl SignalType for bool {}

impl From<i32> for Value {
    fn from(val: i32) -> Value {
        Value::Int(val)
    }
}
impl SignalType for i32 {}


impl From<f32> for Value {
    fn from(val: f32) -> Value {
        Value::Float(val)
    }
}
impl SignalType for f32 {}

