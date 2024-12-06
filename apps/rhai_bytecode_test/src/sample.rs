use rhai_bytecode::{self, DynamicBasicValue, DynamicValue};

/// Simple that only supports a subset of the types (bool, integer, float, array) that are supported by Rhai.
#[derive(Clone,Debug, serde::Serialize, serde::Deserialize)]
//#[serde(untagged)]
pub(crate) enum SimpleBasicValue {
    #[serde(rename="U")]
    Unit,
    #[serde(rename="B")]
    Bool(bool),
    #[serde(rename="I")]
    Integer(rhai_bytecode::INT),
    #[serde(rename="F")]
    Float(rhai_bytecode::FLOAT),
    #[serde(rename="A")]
    Array(rhai_bytecode::VEC<SimpleBasicValue>),
    #[serde(rename="R")]
    Range(rhai_bytecode::INT,rhai_bytecode::INT),
}

impl DynamicBasicValue for SimpleBasicValue {
    fn from_dynamic(dynamic: &rhai_bytecode::rhai::Dynamic) -> anyhow::Result<Self> {
        if dynamic.is_unit() {
            return Ok(Self::Unit);
        } else if dynamic.is_bool() {
            match dynamic.as_bool() {
                Ok(v) => {
                    return Ok(Self::Bool(v));
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert rhai::Dynamic to bool!");
                }
            }
        } else if dynamic.is_int() {
            match dynamic.as_int() {
                Ok(v) => {
                    return Ok(Self::Integer(v));
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert rhai::Dynamic to int!");
                }
            }
        } else if dynamic.is_float() {
            match dynamic.as_float() {
                Ok(v) => {
                    return Ok(Self::Float(v));
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert rhai::Dynamic to float!");
                }
            }
        } else if dynamic.is_array() {
            match dynamic.as_array_ref() {
                Ok(ary) => {
                    let mut vec=rhai_bytecode::VEC::<Self>::with_capacity(ary.len());
                    for item in ary.iter() {
                        vec.push(Self::from_dynamic(item)?);
                    }
                    return Ok(Self::Array(vec));
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert rhai::Dynamic to array!");
                }
            }
        }else if dynamic.type_id()== std::any::TypeId::of::<std::ops::Range<rhai_bytecode::INT>>() {
            match dynamic.clone().try_cast_result::<std::ops::Range<rhai_bytecode::INT>>() {
                Ok(range) => {
                    let l=range.end-range.start;
                    if l < 0 {
                        anyhow::bail!("Range \"{:?}\"'s start is greater than its end!",range);
                    } else {
                        return Ok(Self::Range(range.start,l));
                    }
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert rhai::Dynamic to range!");
                }
            }
        }else if dynamic.type_id()== std::any::TypeId::of::<std::ops::RangeInclusive<rhai_bytecode::INT>>() {
            match dynamic.clone().try_cast_result::<std::ops::RangeInclusive<rhai_bytecode::INT>>() {
                Ok(range) => {
                    // I think this is enough, another type is not needed.
                    let l=range.end()-range.start();
                    if l < 0 {
                        anyhow::bail!("Range \"{:?}\"'s start is greater than its end!",range);
                    } else {
                        return Ok(Self::Range(*range.start(),l+1));
                    }
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert rhai::Dynamic to range!");
                }
            }
        }else{
            anyhow::bail!("Unsupported type \"{:?}\"!", dynamic.type_name());
        }
    }
    fn from_unit() -> Self{
        return Self::Unit;
    }
    fn from_bool(v:bool) -> anyhow::Result<Self> {
        return Ok(Self::Bool(v));
    }
    fn from_integer(v:rhai_bytecode::INT) -> anyhow::Result<Self> {
        return Ok(Self::Integer(v));
    }
    fn from_float(v:rhai_bytecode::FLOAT) -> anyhow::Result<Self> {
        return Ok(Self::Float(v));
    }
    fn from_char(_v:char) -> anyhow::Result<Self> {
        anyhow::bail!("Unsupported type \"char\"!");
    }
    fn from_string(_v:&String) -> anyhow::Result<Self> {
        anyhow::bail!("Unsupported type \"string\"!");
    }
    fn index_into(&self,ind:rhai_bytecode::SIZE)->anyhow::Result<Self> {
        match self {
            Self::Array(vec) => {
                let index= ind as usize;
                if index >= vec.len() {
                    anyhow::bail!("Index \"{}\" out of range!",ind);
                } else {
                    return Ok(vec[index].clone());
                }
            }
            _ => {
                anyhow::bail!("Cannot index into \"{:?}\"!",self);
            }
        }
    }
    fn multi_index_into(&self,inds:&[rhai_bytecode::SIZE])->anyhow::Result<&Self> {
        if inds.is_empty() {
            return Ok(self);
        } else{
            let mut current=self;
            for ind in inds {
                match current {
                    Self::Array(vec) => {
                        let index= *ind as usize;
                        if index >= vec.len() {
                            anyhow::bail!("Index \"{}\" out of range!",index);
                        } else {
                            current=&vec[index];
                        }
                    }
                    _ => {
                        anyhow::bail!("Cannot index into \"{:?}\"!",current);
                    }
                }
            }
            return Ok(current);
        }
    }
    fn is_unit(&self) -> bool{
        match self {
            Self::Unit => {return true;}
            _ => {return false;}
        }
    }
    fn to_bool(&self) -> anyhow::Result<bool>{
        match self {
            Self::Bool(v) => {
                return Ok(*v);
            }
            Self::Integer(v) => {
                return Ok(*v != 0);
            }
            Self::Float(v) => {
                return Ok(!v.is_nan() && *v != 0.0);
            }
            _ => {
                anyhow::bail!("Cannot convert \"{:?}\" to bool!", self);
            }
        }
    }
    fn to_size(&self) -> anyhow::Result<rhai_bytecode::SIZE>{
        match self {
            Self::Integer(v) => {
                return Ok(*v as rhai_bytecode::SIZE);
            }
            _ => {
                anyhow::bail!("Cannot convert \"{:?}\" to size!", self);
            }
        }
    }
    fn set_value(&mut self,inds:&[rhai_bytecode::SIZE],value:Self)->anyhow::Result<()> {
        if inds.is_empty() {
            *self=value;
            return Ok(());
        }else{
            let mut current=self;
            for ind in inds{
                match current {
                    Self::Array(vec) => {
                        let index= *ind as usize;
                        if index >= vec.len() {
                            anyhow::bail!("Index \"{}\" out of range!",index);
                        } else {
                            current=&mut vec[index];
                        }
                    }
                    _ => {
                        anyhow::bail!("Cannot index into \"{:?}\"!",current);
                    }
                }
            }
            *current=value;
            return Ok(());
        }
    }
    fn multi_index_into_mut(&mut self,inds:&[rhai_bytecode::SIZE])->anyhow::Result<&mut Self> {
        if inds.is_empty() {
            return Ok(self);
        } else{
            let mut current=self;
            for ind in inds {
                match current {
                    Self::Array(vec) => {
                        let index= *ind as usize;
                        if index >= vec.len() {
                            anyhow::bail!("Index \"{}\" out of range!",index);
                        } else {
                            current=&mut vec[index];
                        }
                    }
                    _ => {
                        anyhow::bail!("Cannot index into \"{:?}\"!",current);
                    }
                }
            }
            return Ok(current);
        }
    }
    fn iter(&self,index:rhai_bytecode::SIZE) -> anyhow::Result<Option<Self>> {
        match self {
            Self::Array(vec) => {
                let ind= index as usize;
                if ind >= vec.len() {
                    return Ok(None);
                } else {
                    return Ok(Some(vec[ind].clone()));
                }
            }
            Self::Range(start, len) => {
                let offset = index as rhai_bytecode::INT;
                if offset >= *len {
                    return Ok(None);
                }else {
                    return Ok(Some(Self::Integer(*start+offset)));
                }
            }
            _=> {
                anyhow::bail!("Cannot iterate over \"{:?}\"!",self);
            }
        }
    }
    
    fn from_array(v:rhai_bytecode::VEC<Self>) -> anyhow::Result<Self> {
        return Ok(Self::Array(v));
    }
}

impl SimpleBasicValue {
    fn not(&self) -> anyhow::Result<Self> {
        match self {
            Self::Unit => {
                return Ok(Self::Bool(true));
            }
            Self::Bool(va) => {
                return Ok(Self::Bool(!va));
            }
            Self::Integer(va) => {
                return Ok(Self::Bool(*va == 0));
            }
            Self::Float(va) => {
                return Ok(Self::Bool(va.is_nan()||*va == 0.0));
            }
            _=>{
                anyhow::bail!(
                    "Operator \"!\" can not be applied to \"{:?}\"!",
                    self
                );
            }
        }
    }
    fn negative(&self) -> anyhow::Result<Self> {
        match self {
            SimpleBasicValue::Integer(va) => {
                return Ok(SimpleBasicValue::Integer(-va));
            }
            SimpleBasicValue::Float(va) => {
                return Ok(SimpleBasicValue::Float(-va));
            }
            _=>{
                anyhow::bail!(
                    "Operator \"-\" can not be applied to \"{:?}\"!",
                    self
                );
            }
        }
    }
    fn add(&self,other: &Self) -> anyhow::Result<Self> {
        match (self,other) {
            (Self::Integer(va), Self::Integer(vb)) => {
                return Ok(Self::Integer(*va + *vb));
            }
            (Self::Integer(va), Self::Float(vb)) => {
                return Ok(Self::Float((*va as rhai_bytecode::FLOAT) + *vb));
            }
            (Self::Float(va), Self::Integer(vb)) => {
                return Ok(Self::Float(*va + (*vb as rhai_bytecode::FLOAT)));
            }
            (Self::Float(va), Self::Float(vb)) => {
                return Ok(Self::Float(*va + *vb));
            }
            _ => {
                anyhow::bail!(
                    "Operator \"+\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    self,
                    other
                );
            }
        }
    }
    fn subtract(&self,other: &Self) -> anyhow::Result<Self> {
        match (self,other) {
            (Self::Integer(va), Self::Integer(vb)) => {
                return Ok(Self::Integer(*va - *vb));
            }
            (Self::Integer(va), Self::Float(vb)) => {
                return Ok(Self::Float((*va as rhai_bytecode::FLOAT) - *vb));
            }
            (Self::Float(va), Self::Integer(vb)) => {
                return Ok(Self::Float(*va - (*vb as rhai_bytecode::FLOAT)));
            }
            (Self::Float(va), Self::Float(vb)) => {
                return Ok(Self::Float(*va - *vb));
            }
            _ => {
                anyhow::bail!(
                    "Operator \"-\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    self,
                    other
                );
            }
        }
    }
    fn multiply(&self,other: &Self) -> anyhow::Result<Self> {
        match (self,other) {
            (Self::Integer(va), Self::Integer(vb)) => {
                return Ok(Self::Integer(*va * *vb));
            }
            (Self::Integer(va), Self::Float(vb)) => {
                return Ok(Self::Float((*va as rhai_bytecode::FLOAT) * *vb));
            }
            (Self::Float(va), Self::Integer(vb)) => {
                return Ok(Self::Float(*va * (*vb as rhai_bytecode::FLOAT)));
            }
            (Self::Float(va), Self::Float(vb)) => {
                return Ok(Self::Float(*va * *vb));
            }
            _ => {
                anyhow::bail!(
                    "Operator \"*\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    self,
                    other
                );
            }
        }
    }
    fn divide(&self,other: &Self) -> anyhow::Result<Self> {
        match (self,other) {
            (Self::Integer(va), Self::Integer(vb)) => {
                if *vb == 0 {
                    anyhow::bail!("Divisor can not be zero!");
                }else{
                    return Ok(Self::Integer(*va / *vb));
                }
            }
            (Self::Integer(va), Self::Float(vb)) => {
                return Ok(Self::Float((*va as rhai_bytecode::FLOAT) / *vb));
            }
            (Self::Float(va), Self::Integer(vb)) => {
                return Ok(Self::Float(*va / (*vb as rhai_bytecode::FLOAT)));
            }
            (Self::Float(va), Self::Float(vb)) => {
                return Ok(Self::Float(*va / *vb));
            }
            _ => {
                anyhow::bail!(
                    "Operator \"/\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    self,
                    other
                );
            }
        }
    }
    fn modulus(&self,other: &Self) -> anyhow::Result<Self> {
        match (self,other) {
            (Self::Integer(va), Self::Integer(vb)) => {
                if *vb == 0 {
                    anyhow::bail!("Divisor can not be zero!");
                }else{
                    return Ok(Self::Integer(*va % *vb));
                }
            }
            _ => {
                anyhow::bail!(
                    "Operator \"%\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    self,
                    other
                );
            }
        }
    }
    fn power(&self,other: &Self) -> anyhow::Result<Self> {
        match (self,other) {
            (Self::Integer(va), Self::Integer(vb)) => {
                match (*vb).try_into() {
                    Ok(v) => {
                        return Ok(Self::Integer(va.pow(v)));
                    }
                    Err(_) => {
                        return Ok(Self::Float((*va as rhai_bytecode::FLOAT).powf(*vb as rhai_bytecode::FLOAT)));
                    }
                }
            }
            (Self::Integer(va), Self::Float(vb)) => {
                return Ok(Self::Float((*va as rhai_bytecode::FLOAT).powf(*vb)));
            }
            (Self::Float(va), Self::Integer(vb)) => {
                return Ok(Self::Float(va.powf(*vb as rhai_bytecode::FLOAT)));
            }
            (Self::Float(va), Self::Float(vb)) => {
                return Ok(Self::Float(va.powf(*vb)));
            }
            _ => {
                anyhow::bail!(
                    "Operator \"^\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    self,
                    other
                );
            }
        }
    }
    fn equals(&self,other: &Self) -> anyhow::Result<bool> {
        match (self,other){
            (Self::Unit, Self::Unit) => {
                return Ok(true);
            }
            (Self::Bool(va), Self::Bool(vb)) => {
                return Ok(*va == *vb);
            }
            (Self::Integer(va), Self::Integer(vb)) => {
                return Ok(*va == *vb);
            }
            (Self::Integer(va), Self::Float(vb)) => {
                return Ok((*va as rhai_bytecode::FLOAT) == *vb);
            }
            (Self::Float(va), Self::Integer(vb)) => {
                return Ok(*va == (*vb as rhai_bytecode::FLOAT));
            }
            (Self::Float(va), Self::Float(vb)) => {
                return Ok(*va == *vb);
            }
            _ => {
                anyhow::bail!(
                    "Operator \"==\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    self,
                    other
                );
            }
        }
    }
    fn not_equals(&self,other: &Self) -> anyhow::Result<bool> {
        match (self,other){
            (Self::Unit, Self::Unit) => {
                return Ok(false);
            }
            (Self::Bool(va), Self::Bool(vb)) => {
                return Ok(*va != *vb);
            }
            (Self::Integer(va), Self::Integer(vb)) => {
                return Ok(*va != *vb);
            }
            (Self::Integer(va), Self::Float(vb)) => {
                return Ok((*va as rhai_bytecode::FLOAT) != *vb);
            }
            (Self::Float(va), Self::Integer(vb)) => {
                return Ok(*va != (*vb as rhai_bytecode::FLOAT));
            }
            (Self::Float(va), Self::Float(vb)) => {
                return Ok(*va != *vb);
            }
            _ => {
                anyhow::bail!(
                    "Operator \"!=\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    self,
                    other
                );
            }
        }
    }
    fn less_than(&self,other: &Self) -> anyhow::Result<bool> {
        match (self,other){
            (Self::Unit, Self::Unit) => {
                return Ok(false);
            }
            (Self::Bool(va), Self::Bool(vb)) => {
                return Ok(*va < *vb);
            }
            (Self::Integer(va), Self::Integer(vb)) => {
                return Ok(*va < *vb);
            }
            (Self::Integer(va), Self::Float(vb)) => {
                return Ok((*va as rhai_bytecode::FLOAT) < *vb);
            }
            (Self::Float(va), Self::Integer(vb)) => {
                return Ok(*va < (*vb as rhai_bytecode::FLOAT));
            }
            (Self::Float(va), Self::Float(vb)) => {
                return Ok(*va < *vb);
            }
            _ => {
                anyhow::bail!(
                    "Operator \"<\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    self,
                    other
                );
            }
        }
    }
    fn greater_than(&self,other: &Self) -> anyhow::Result<bool> {
        match (self,other){
            (Self::Unit, Self::Unit) => {
                return Ok(false);
            }
            (Self::Bool(va), Self::Bool(vb)) => {
                return Ok(*va > *vb);
            }
            (Self::Integer(va), Self::Integer(vb)) => {
                return Ok(*va > *vb);
            }
            (Self::Integer(va), Self::Float(vb)) => {
                return Ok((*va as rhai_bytecode::FLOAT) > *vb);
            }
            (Self::Float(va), Self::Integer(vb)) => {
                return Ok(*va > (*vb as rhai_bytecode::FLOAT));
            }
            (Self::Float(va), Self::Float(vb)) => {
                return Ok(*va > *vb);
            }
            _ => {
                anyhow::bail!(
                    "Operator \">\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    self,
                    other
                );
            }
        }
    }
    fn less_equal_to(&self,other: &Self) -> anyhow::Result<bool> {
        match (self,other){
            (Self::Unit, Self::Unit) => {
                return Ok(true);
            }
            (Self::Bool(va), Self::Bool(vb)) => {
                return Ok(*va <= *vb);
            }
            (Self::Integer(va), Self::Integer(vb)) => {
                return Ok(*va <= *vb);
            }
            (Self::Integer(va), Self::Float(vb)) => {
                return Ok((*va as rhai_bytecode::FLOAT) <= *vb);
            }
            (Self::Float(va), Self::Integer(vb)) => {
                return Ok(*va <= (*vb as rhai_bytecode::FLOAT));
            }
            (Self::Float(va), Self::Float(vb)) => {
                return Ok(*va <= *vb);
            }
            _ => {
                anyhow::bail!(
                    "Operator \"<=\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    self,
                    other
                );
            }
        }
    }
    fn greater_equal_to(&self,other: &Self) -> anyhow::Result<bool> {
        match (self,other){
            (Self::Unit, Self::Unit) => {
                return Ok(true);
            }
            (Self::Bool(va), Self::Bool(vb)) => {
                return Ok(*va >= *vb);
            }
            (Self::Integer(va), Self::Integer(vb)) => {
                return Ok(*va >= *vb);
            }
            (Self::Integer(va), Self::Float(vb)) => {
                return Ok((*va as rhai_bytecode::FLOAT) >= *vb);
            }
            (Self::Float(va), Self::Integer(vb)) => {
                return Ok(*va >= (*vb as rhai_bytecode::FLOAT));
            }
            (Self::Float(va), Self::Float(vb)) => {
                return Ok(*va >= *vb);
            }
            _ => {
                anyhow::bail!(
                    "Operator \">=\" can not be applied to \"{:?}\" and \"{:?}\"!",
                    self,
                    other
                );
            }
        }
    }
}

fn not(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    return Ok(DynamicValue::Basic(args[0].deref(variables)?.not()?));
}

fn add(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    return Ok(DynamicValue::Basic(args[0].deref(variables)?.add(args[1].deref(variables)?)?));
}

fn subtract(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    if args.len() == 2 {
        return Ok(DynamicValue::Basic(args[0].deref(variables)?.subtract(args[1].deref(variables)?)?));
    }else{
        return Ok(DynamicValue::Basic(args[0].deref(variables)?.negative()?));
    }
}

fn multiply(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    return Ok(DynamicValue::Basic(args[0].deref(variables)?.multiply(args[1].deref(variables)?)?));
}

fn divide(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    return Ok(DynamicValue::Basic(args[0].deref(variables)?.divide(args[1].deref(variables)?)?));
}

fn modulus(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    return Ok(DynamicValue::Basic(args[0].deref(variables)?.modulus(args[1].deref(variables)?)?));
}

fn power(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    return Ok(DynamicValue::Basic(args[0].deref(variables)?.power(args[1].deref(variables)?)?));
}

fn assign(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    let rhs=args[1].get_value(variables)?;
    args[0].set_value(variables, rhs)?;
    return Ok(args[0].clone());
}

fn add_assign(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    let res=args[0].deref(variables)?.add(args[1].deref(variables)?)?;
    args[0].set_value(variables,res)?;
    return Ok(args[0].clone());
}

fn subtract_assign(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    let res=args[0].deref(variables)?.subtract(args[1].deref(variables)?)?;
    args[0].set_value(variables,res)?;
    return Ok(args[0].clone());
}

fn multiply_assign(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    let res=args[0].deref(variables)?.multiply(args[1].deref(variables)?)?;
    args[0].set_value(variables,res)?;
    return Ok(args[0].clone());
}

fn divide_assign(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    let res=args[0].deref(variables)?.divide(args[1].deref(variables)?)?;
    args[0].set_value(variables,res)?;
    return Ok(args[0].clone());
}

fn equals(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    return Ok(DynamicValue::Basic(SimpleBasicValue::Bool(args[0].deref(variables)?.equals(args[1].deref(variables)?)?)));
}

fn not_equals(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    return Ok(DynamicValue::Basic(SimpleBasicValue::Bool(args[0].deref(variables)?.not_equals(args[1].deref(variables)?)?)));
}

fn less_than(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    return Ok(DynamicValue::Basic(SimpleBasicValue::Bool(args[0].deref(variables)?.less_than(args[1].deref(variables)?)?)));
}

fn greater_than(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    return Ok(DynamicValue::Basic(SimpleBasicValue::Bool(args[0].deref(variables)?.greater_than(args[1].deref(variables)?)?)));
}

fn less_equal_to(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    return Ok(DynamicValue::Basic(SimpleBasicValue::Bool(args[0].deref(variables)?.less_equal_to(args[1].deref(variables)?)?)));
}

fn greater_equal_to(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    return Ok(DynamicValue::Basic(SimpleBasicValue::Bool(args[0].deref(variables)?.greater_equal_to(args[1].deref(variables)?)?)));
}

fn range(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    let a= args[0].deref(variables)?;
    let b = args[1].deref(variables)?;
    match (a,b) {
        (SimpleBasicValue::Integer(va), SimpleBasicValue::Integer(vb)) => {
            let l = *vb-*va;
            if l < 0{
                anyhow::bail!("Range start \"{}\" is greater than end \"{}\"!", va, vb);
            } else{
                return Ok(DynamicValue::Basic(SimpleBasicValue::Range(*va,l)));
            }
        }
        _=>{
            anyhow::bail!(
                "Operator \"..\" can not be applied to \"{:?}\" and \"{:?}\"!",
                a,
                b
            );
        }
    }
}

fn range_inclusive(args: &[DynamicValue<SimpleBasicValue>],variables: &mut Vec<SimpleBasicValue>) -> anyhow::Result<DynamicValue<SimpleBasicValue>> {
    let a= args[0].deref(variables)?;
    let b = args[1].deref(variables)?;
    match (a,b) {
        (SimpleBasicValue::Integer(va), SimpleBasicValue::Integer(vb)) => {
            let l = *vb-*va;
            if l < 0{
                anyhow::bail!("Range start \"{}\" is greater than end \"{}\"!", va, vb);
            } else{
                return Ok(DynamicValue::Basic(SimpleBasicValue::Range(*va,l+1)));
            }
        }
        _=>{
            anyhow::bail!(
                "Operator \"..=\" can not be applied to \"{:?}\" and \"{:?}\"!",
                a,
                b
            );
        }
    }
}

pub(crate) fn new_executer() -> anyhow::Result<rhai_bytecode::Executer<SimpleBasicValue>> {
    let mut executer = rhai_bytecode::Executer::<SimpleBasicValue>::new();
    executer.add_fn("!", not,1,1)?;
    executer.add_fn("+", add,2,2)?;
    executer.add_fn("-", subtract,1,2)?;
    executer.add_fn("*", multiply,2,2)?;
    executer.add_fn("/", divide,2,2)?;
    executer.add_fn("%", modulus,2,2)?;
    executer.add_fn("^", power,2,2)?;
    executer.add_fn("=", assign,2,2)?;
    executer.add_fn("+=", add_assign,2,2)?;
    executer.add_fn("-=", subtract_assign,2,2)?;
    executer.add_fn("*=", multiply_assign,2,2)?;
    executer.add_fn("/=", divide_assign,2,2)?;
    executer.add_fn("==", equals,2,2)?;
    executer.add_fn("!=", not_equals,2,2)?;
    executer.add_fn("<", less_than,2,2)?;
    executer.add_fn(">", greater_than,2,2)?;
    executer.add_fn("<=", less_equal_to,2,2)?;
    executer.add_fn(">=", greater_equal_to,2,2)?;
    executer.add_fn("..", range,2,2)?;
    executer.add_fn("..=", range_inclusive,2,2)?;
    return Ok(executer);
}