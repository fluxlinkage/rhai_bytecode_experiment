use std::ops::Deref;

use rhai_bytecode::{self, DynamicValue};

#[derive(Clone, Debug)]
pub(crate) enum SimpleDynamicValue {
    Unit,
    Bool(bool),
    Integer(rhai_bytecode::INT),
    Float(rhai_bytecode::FLOAT),
    Array(Vec<std::rc::Rc<std::cell::RefCell<SimpleDynamicValue>>>),
    Reference(std::rc::Rc<std::cell::RefCell<SimpleDynamicValue>>)
}

impl rhai_bytecode::DynamicValue for SimpleDynamicValue {
    fn from_dynamic(dynamic: rhai_bytecode::rhai::Dynamic) -> anyhow::Result<Self> {
        if dynamic.is_unit() {
            return Self::from_unit();
        } else if dynamic.is_bool() {
            match dynamic.as_bool() {
                Ok(v) => {
                    return Self::from_bool(v);
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert Dynamic to bool!");
                }
            }
        } else if dynamic.is_char() {
            match dynamic.as_char() {
                Ok(v) => {
                    return Self::from_char(v);
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert Dynamic to char!");
                }
            }
        } else if dynamic.is_int() {
            match dynamic.as_int() {
                Ok(v) => {
                    return Self::from_integer(v);
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert Dynamic to int!");
                }
            }
        } else if dynamic.is_float() {
            match dynamic.as_float() {
                Ok(v) => {
                    return Self::from_float(v);
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert Dynamic to float!");
                }
            }
        } else if dynamic.is_array() {
            match dynamic.as_array_ref() {
                Ok(ary) => {
                    let mut vec=Vec::<std::rc::Rc<std::cell::RefCell<SimpleDynamicValue>>>::with_capacity(ary.len());
                    for item in ary.iter() {
                        vec.push(std::rc::Rc::new(std::cell::RefCell::new(Self::from_dynamic(item.clone())?)));
                    }
                    return Ok(SimpleDynamicValue::Array(vec));
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert Dynamic to array!");
                }
            }
        }
        anyhow::bail!("Unsupported type: {}", dynamic.type_name());
    }
    
    fn from_variable_ref(var:std::rc::Rc<std::cell::RefCell<Self>>) -> anyhow::Result<Self> {
        return Ok(SimpleDynamicValue::Reference(var));
    }

    fn from_unit() -> anyhow::Result<Self> {
        return Ok(SimpleDynamicValue::Unit);
    }

    fn from_bool(v: bool) -> anyhow::Result<Self> {
        return Ok(SimpleDynamicValue::Bool(v));
    }

    fn from_char(v: char) -> anyhow::Result<Self> {
        // Simplely, we'll just cast the char as an integer.
        return Ok(SimpleDynamicValue::Integer(v as rhai_bytecode::INT));
    }

    fn from_integer(v: rhai_bytecode::INT) -> anyhow::Result<Self> {
        return Ok(SimpleDynamicValue::Integer(v));
    }

    fn from_float(v: rhai_bytecode::FLOAT) -> anyhow::Result<Self> {
        return Ok(SimpleDynamicValue::Float(v));
    }

    fn is_unit(&self) -> anyhow::Result<bool> {
        match self.get_value()? {
            SimpleDynamicValue::Unit => {
                return Ok(true);
            }
            _ => {
                return Ok(false);
            }
        }
    }

    fn to_bool(&self) -> anyhow::Result<bool> {
        match self.get_value()? {
            SimpleDynamicValue::Bool(v) => {
                return Ok(v);
            }
            SimpleDynamicValue::Integer(v) => {
                return Ok(v != 0);
            }
            SimpleDynamicValue::Float(v) => {
                return Ok(!v.is_nan() && v != 0.0);
            }
            _ => {
                anyhow::bail!("Cannot convert \"{:?}\" to bool!", self);
            }
        }
    }

    fn to_size(&self) -> anyhow::Result<rhai_bytecode::OpSize> {
        match self.get_value()? {
            SimpleDynamicValue::Integer(v) => {
                return Ok(v as rhai_bytecode::OpSize);
            }
            _ => {
                anyhow::bail!("Cannot convert \"{:?}\" to size!", self);
            }
        }
    }

    fn get_value(&self) -> anyhow::Result<Self> {
        match self {
            SimpleDynamicValue::Reference(rc) => {
                let a=rc.try_borrow()?;
                return a.get_value();
            }
            _ => {
                return Ok(self.clone());
            }
        }
    }
    
    fn set_value(&self,val:Self) -> anyhow::Result<()> {
        match self{
            SimpleDynamicValue::Reference(rc)=>{
                *(rc.try_borrow_mut()?)=val;
                return Ok(());
            }
            _ => {
                anyhow::bail!("Variable \"{:?}\" is not a reference!", self);
            }
        }
    }

    fn enter_index(&mut self, ind: rhai_bytecode::OpSize) -> anyhow::Result<()> {
        let immutable_self: &SimpleDynamicValue=self;
        let new_value=
        match immutable_self {
            SimpleDynamicValue::Array(vec) => {
                let index= ind as usize;
                if index >= vec.len() {
                    anyhow::bail!("Index \"{}\" out of bounds!",ind);
                }else{
                    //vec[index].try_borrow()?.clone()
                    vec[index].clone()
                }
            }
            SimpleDynamicValue::Reference(rc)=>{
                match rc.try_borrow()?.deref() {
                    SimpleDynamicValue::Array(vec) => {
                        let index= ind as usize;
                        if index >= vec.len() {
                            anyhow::bail!("Index \"{}\" out of bounds!",ind);
                        }else{
                            //vec[index].try_borrow()?.clone()
                            vec[index].clone()
                        }
                    }
                    _ => {
                        anyhow::bail!("Variable \"{:?}\" does not have index!", self);
                    }
                }
            }
            _ => {
                anyhow::bail!("Variable \"{:?}\" does not have index!", self);
            }
        };
        *self=SimpleDynamicValue::Reference(new_value);
        return Ok(());
    }
}

fn not(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 1 {
        anyhow::bail!(
            "Operator \"!\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let a = args[0].get_value()?;
        match a {
            SimpleDynamicValue::Unit => {
                return Ok(SimpleDynamicValue::Bool(true));
            }
            SimpleDynamicValue::Bool(va) => {
                return Ok(SimpleDynamicValue::Bool(!va));
            }
            SimpleDynamicValue::Integer(va)=>{
                return Ok(SimpleDynamicValue::Bool(va==0));
            }
            SimpleDynamicValue::Float(va) => {
                return Ok(SimpleDynamicValue::Bool(va.is_nan()||va==0.0));
            }
            _=> {
                anyhow::bail!(
                    "Operator \"!\" can not be applied to \"{:?}\"!",
                    args[0].get_value()?
                );
            }
        }
    }
}

fn add(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"+\" needs 2 arguments, but {} provided!",
            args.len()
        );
    } else {
        let a = args[0].get_value()?;
        let b = args[1].get_value()?;
        match (a, b) {
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Integer(va + vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Float(va as rhai_bytecode::FLOAT + vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Float(va + vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Float(va + vb as rhai_bytecode::FLOAT));
            }
            _ => {
                anyhow::bail!(
                    "Operator \"+\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    args[0].get_value()?,
                    args[1].get_value()?
                );
            }
        }
    }
}

fn subtract(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"-\" needs 2 arguments, but {} provided!",
            args.len()
        );
    } else {
        let a = args[0].get_value()?;
        let b = args[1].get_value()?;
        match (a, b) {
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Integer(va - vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Float(va as rhai_bytecode::FLOAT - vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Float(va - vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Float(va - vb as rhai_bytecode::FLOAT));
            }
            _ => {
                anyhow::bail!(
                    "Operator \"-\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    args[0].get_value()?,
                    args[1].get_value()?
                );
            }
        }
    }
}

fn multiply(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue>{
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"*\" needs 2 arguments, but {} provided!",
            args.len()
        );
    } else{
        let a = args[0].get_value()?;
        let b = args[1].get_value()?;
        match (a,b) {
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Integer(va * vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Float(va as rhai_bytecode::FLOAT * vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Float(va * vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Float(va * vb as rhai_bytecode::FLOAT));
            }
            _ => {
                anyhow::bail!(
                    "Operator \"*\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    args[0].get_value()?,
                    args[1].get_value()?
                );
            }
        }
    }
}

fn divide(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"/\" needs 2 arguments, but {} provided!",
            args.len()
        );
    } else{
        let a = args[0].get_value()?;
        let b = args[1].get_value()?;
        match (a,b) {
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                if vb == 0 {
                    anyhow::bail!("Divisor can not be zero!");
                }else{
                    return Ok(SimpleDynamicValue::Integer(va / vb));
                }
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Float(va as rhai_bytecode::FLOAT / vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Float(va / vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Float(va / vb as rhai_bytecode::FLOAT));
            }
            _ => {
                anyhow::bail!(
                    "Operator \"/\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    args[0].get_value()?,
                    args[1].get_value()?
                );
            }
        }
    }
}

fn modulus(args: &Vec<SimpleDynamicValue>)-> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"%\" needs 2 arguments, but {} provided!",
            args.len()
        );
    } else{
        let a = args[0].get_value()?;
        let b = args[1].get_value()?;
        match (a,b){
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                if vb == 0 {
                    anyhow::bail!("Divisor can not be zero!");
                }else{
                    return Ok(SimpleDynamicValue::Integer(va % vb));
                }
            }
            _ => {
                anyhow::bail!(
                    "Operator \"%\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    args[0].get_value()?,
                    args[1].get_value()?
                );
            }
        }
    }
}

fn power(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"^\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let a = args[0].get_value()?;
        let b = args[1].get_value()?;
        match (a,b){
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                match vb.try_into() {
                    Ok(v) => {
                        return Ok(SimpleDynamicValue::Integer(va.pow(v)));
                    }
                    Err(_) => {
                        return Ok(SimpleDynamicValue::Float((va as rhai_bytecode::FLOAT).powf(vb as rhai_bytecode::FLOAT)));
                    }
                }
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Float((va as rhai_bytecode::FLOAT).powf(vb)));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Float(va.powf(vb)));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Float(va.powf(vb as rhai_bytecode::FLOAT)));
            }
            _ => {
                anyhow::bail!(
                    "Operator \"^\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    args[0].get_value()?,
                    args[1].get_value()?
                );
            }
        }
    }
}

fn assign(args: &Vec<SimpleDynamicValue>,) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"=\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let res= args[1].get_value()?;
        args[0].set_value(res.clone())?;
        return Ok(res);
    }
}

fn add_assign(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"+=\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let res=add(args)?;
        args[0].set_value(res.clone())?;
        return Ok(res);
    }
}

fn subtract_assign(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"-=\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let res=subtract(args)?;
        args[0].set_value(res.clone())?;
        return Ok(res);
    }
}

fn multiply_assign(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"*=\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let res=multiply(args)?;
        args[0].set_value(res.clone())?;
        return Ok(res);
    }
}

fn divide_assign(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"/=\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let res=divide(args)?;
        args[0].set_value(res.clone())?;
        return Ok(res);
    }
}

fn equals(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"==\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let a = args[0].get_value()?;
        let b = args[1].get_value()?;
        match (a,b) {
            (SimpleDynamicValue::Unit, SimpleDynamicValue::Unit) => {
                return Ok(SimpleDynamicValue::Bool(true));
            }
            (SimpleDynamicValue::Bool(va), SimpleDynamicValue::Bool(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va==vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va == vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool((va as f64) == vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va == (vb as f64)));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va == vb));
            }
            _=>{
                anyhow::bail!(
                    "Operator \"==\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    args[0].get_value()?,
                    args[1].get_value()?
                );
            }
        }
    }
}

fn not_equals(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"!=\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let a = args[0].get_value()?;
        let b = args[1].get_value()?;
        match (a,b) {
            (SimpleDynamicValue::Unit, SimpleDynamicValue::Unit) => {
                return Ok(SimpleDynamicValue::Bool(false));
            }
            (SimpleDynamicValue::Bool(va), SimpleDynamicValue::Bool(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va!=vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va != vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool((va as f64) != vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va != (vb as f64)));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va != vb));
            }
            _=>{
                anyhow::bail!(
                    "Operator \"!=\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    args[0].get_value()?,
                    args[1].get_value()?
                );
            }
        }
    }
}

fn less_than(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"<\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let a = args[0].get_value()?;
        let b = args[1].get_value()?;
        match (a,b) {
            (SimpleDynamicValue::Unit, SimpleDynamicValue::Unit) => {
                return Ok(SimpleDynamicValue::Bool(false));
            }
            (SimpleDynamicValue::Bool(va), SimpleDynamicValue::Bool(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va<vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va < vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool((va as f64) < vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va < (vb as f64)));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va < vb));
            }
            _=>{
                anyhow::bail!(
                    "Operator \"<\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    args[0].get_value()?,
                    args[1].get_value()?
                );
            }
        }
    }
}

fn greater_than(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \">\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let a = args[0].get_value()?;
        let b = args[1].get_value()?;
        match (a,b) {
            (SimpleDynamicValue::Unit, SimpleDynamicValue::Unit) => {
                return Ok(SimpleDynamicValue::Bool(false));
            }
            (SimpleDynamicValue::Bool(va), SimpleDynamicValue::Bool(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va>vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va > vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool((va as f64) > vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va > (vb as f64)));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va > vb));
            }
            _=>{
                anyhow::bail!(
                    "Operator \">\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    args[0].get_value()?,
                    args[1].get_value()?
                );
            }
        }
    }
}

fn less_equal_to(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue>{
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"<=\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let a = args[0].get_value()?;
        let b = args[1].get_value()?;
        match (a,b) {
            (SimpleDynamicValue::Unit, SimpleDynamicValue::Unit) => {
                return Ok(SimpleDynamicValue::Bool(true));
            }
            (SimpleDynamicValue::Bool(va), SimpleDynamicValue::Bool(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va<=vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va <= vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool((va as f64) <= vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va <= (vb as f64)));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va <= vb));
            }
            _=>{
                anyhow::bail!(
                    "Operator \"<=\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    args[0].get_value()?,
                    args[1].get_value()?
                );
            }
        }
    }
}

fn greater_equal_to(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue>{
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \">=\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let a = args[0].get_value()?;
        let b = args[1].get_value()?;
        match (a,b) {
            (SimpleDynamicValue::Unit, SimpleDynamicValue::Unit) => {
                return Ok(SimpleDynamicValue::Bool(true));
            }
            (SimpleDynamicValue::Bool(va), SimpleDynamicValue::Bool(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va>=vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va >= vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool((va as f64) >= vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va >= (vb as f64)));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool(va >= vb));
            }
            _=>{
                anyhow::bail!(
                    "Operator \">=\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    args[0].get_value()?,
                    args[1].get_value()?
                );
            }
        }
    }
}

pub(crate) fn new_executer() -> anyhow::Result<rhai_bytecode::Executer<SimpleDynamicValue>> {
    let mut executer = rhai_bytecode::Executer::<SimpleDynamicValue>::new();
    executer.add_fn("!", not)?;
    executer.add_fn("+", add)?;
    executer.add_fn("-", subtract)?;
    executer.add_fn("*", multiply)?;
    executer.add_fn("/", divide)?;
    executer.add_fn("%", modulus)?;
    executer.add_fn("^", power)?;
    executer.add_fn("=", assign)?;
    executer.add_fn("+=", add_assign)?;
    executer.add_fn("-=", subtract_assign)?;
    executer.add_fn("*=", multiply_assign)?;
    executer.add_fn("/=", divide_assign)?;
    executer.add_fn("==", equals)?;
    executer.add_fn("!=", not_equals)?;
    executer.add_fn("<", less_than)?;
    executer.add_fn(">", greater_than)?;
    executer.add_fn("<=", less_equal_to)?;
    executer.add_fn(">=", greater_equal_to)?;
    return Ok(executer);
}