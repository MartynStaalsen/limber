pub enum Value {
    Bool(bool),
    Int(i32),
    Float(f32)
}

pub trait SignalType: Into<Value> + TryFrom<Value> {
}

impl From<bool> for Value { // implements Into for SignalType
    fn from(val: bool) -> Value {
        Value::Bool(val)
    }
}
impl TryFrom<Value> for bool{
    type Error = &'static str;

    fn try_from(val: Value) -> Result<Self, Self::Error> {
        match val {
            Value::Bool(v) => Ok(v),
            _ => Err("Value was not Bool")
        }
    }
}
impl SignalType for bool {}

impl TryFrom<Value> for i32{
    type Error = &'static str;

    fn try_from(val: Value) -> Result<Self, Self::Error> {
        match val {
            Value::Int(i) => Ok(i),
            _ => Err("Value was not an Int")
        }
    }
}
impl From<i32> for Value {
    fn from(val: i32) -> Value {
        Value::Int(val)
    }
}
impl SignalType for i32 {}


impl TryFrom<Value> for f32{
    type Error = &'static str;

    fn try_from(val: Value) -> Result<Self, Self::Error> {
        match val {
            Value::Float(f) => Ok(f),
            _ => Err("Value was not a Float")
        }
    }
}
impl From<f32> for Value {
    fn from(val: f32) -> Value {
        Value::Float(val)
    }
}
impl SignalType for f32 {}

