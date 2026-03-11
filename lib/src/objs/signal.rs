pub enum Value {
    Bool(bool),
    Int(i32),
    Float(f32)
}

pub trait SignalType: Into<Value> + TryFrom<Value> {
}

macro_rules! map_signal_types {
    ($t:ty, $variant:ident) => {
        impl From<$t> for Value { // implements Into for SignalType
            fn from(val: $t) -> Value {
                Value::$variant(val)
            }
        }
        impl TryFrom<Value> for $t{
            type Error = &'static str;

            fn try_from(val: Value) -> Result<Self, Self::Error> {
                match val {
                    Value::$variant(v) => Ok(v),
                    _ => Err("Value was incorrect variant")
                }
            }
        }
        impl SignalType for $t {}
    }
}

map_signal_types!(bool, Bool);                                                                                                              
map_signal_types!(i32, Int);                                                                                                                
map_signal_types!(f32, Float);  

#[derive(Clone)]
struct SignalReader<T> { //where T: SignalType {
    index: usize,
    phantom: std::marker::PhantomData<T>,
}

struct SignalWriter<T> { // where T: SignalType {
    index: usize,
    phantom: std::marker::PhantomData<T>,
}

