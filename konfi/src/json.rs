use serde_json::{Value, Number, Map};
use crate::eval::Val;

#[derive(Debug)]
pub struct SerializationError {
    pub message: String,
}

pub fn to_json(v: &Val) -> Result<Value, SerializationError> {
    match v {
        Val::Nil => Ok(Value::Null),
        Val::Rec(r) => {
            let mut m = Map::new();
            let r = &*r.borrow();
            for (f, fv) in r.fields.iter() {
                m.insert(f.clone(), to_json(fv)?);
            }
            Ok(Value::Object(m))
        }
        Val::Bool(b) => Ok(Value::Bool(*b)),
        Val::Int(i) => Ok(Value::Number(Number::from(*i))),
        Val::Double(d) => match Number::from_f64(*d) {
            Some(x) => Ok(Value::Number(x)),
            None => Err(SerializationError{message: format!("Cannot serialize Double({})", *d)})
        },
        Val::Str(s) => Ok(Value::String(s.clone())),
        Val::Timestamp(_) => todo!(),
        Val::Duration(_) => todo!(),
    }
}