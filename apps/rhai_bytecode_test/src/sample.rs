use std::cell::RefCell;
use std::rc::Rc;
use rhai_bytecode::{self, DynamicConstant,DynamicValue};

macro_rules! add_int_int {
    ($a:ident, $b:ident) => {
        return Ok(Self::Integer(*$a + *$b));
    }
}
macro_rules! subtract_int_int {
    ($a:ident, $b:ident) => {
        return Ok(Self::Integer(*$a - *$b));
    }
}
macro_rules! multiply_int_int {
    ($a:ident, $b:ident) => {
        return Ok(Self::Integer(*$a * *$b));
    }
}
macro_rules! divide_int_int {
    ($a:ident, $b:ident) => {
        if *$a == 0 {
            anyhow::bail!("Divisor can not be zero!");
        }else{
            return Ok(Self::Integer(*$a / *$b));
        }
    }
}
macro_rules! create_simple_arithmetic{
    ($func_name:ident,$macro_int_int:tt,$op:tt)=>{
        fn $func_name(&self,other: &Self) -> anyhow::Result<Self>{
            match (self,other) {
                (Self::Integer(va), Self::Integer(vb)) => {
                    $macro_int_int!(va,vb);
                }
                (Self::Integer(va), Self::Float(vb)) => {
                    return Ok(Self::Float((*va as rhai_bytecode::FLOAT) $op *vb));
                }
                (Self::Float(va), Self::Integer(vb)) => {
                    return Ok(Self::Float(*va $op (*vb as rhai_bytecode::FLOAT)));
                }
                (Self::Float(va), Self::Float(vb)) => {
                    return Ok(Self::Float(*va $op *vb));
                }
                _ => {
                    anyhow::bail!(
                        "Cannot calculate \"{}\" for \"{:?}\" and \"{:?}\"!",
                        stringify!($op),
                        self,
                        other
                    );
                }
            }
        }
    }
}
macro_rules! create_simple_compare{
    ($func_name:ident,$op:tt)=>{
        fn $func_name(&self,other: &Self) -> anyhow::Result<bool>{
            match (self,other){
                (Self::Unit, Self::Unit) => {
                    return Ok(false $op false);
                }
                (Self::Bool(va), Self::Bool(vb)) => {
                    return Ok(*va $op *vb);
                }
                (Self::Integer(va), Self::Integer(vb)) => {
                    return Ok(*va $op *vb);
                }
                (Self::Integer(va), Self::Float(vb)) => {
                    return Ok((*va as rhai_bytecode::FLOAT) $op *vb);
                }
                (Self::Float(va), Self::Integer(vb)) => {
                    return Ok(*va $op (*vb as rhai_bytecode::FLOAT));
                }
                (Self::Float(va), Self::Float(vb)) => {
                    return Ok(*va $op *vb);
                }
                _ => {
                    anyhow::bail!(
                        "Cannot calculate \"{}\" for \"{:?}\" and \"{:?}\"!",
                        stringify!($op),
                        self,
                        other
                    );
                }
            }
        }
    }
}
macro_rules! create_simple_binary_function {
    ($func_name:ident)=>{
        fn $func_name(args: &[Rc<RefCell<SimpleDynamicValue>>]) -> anyhow::Result<Rc<RefCell<SimpleDynamicValue>>>  {
            let res=args[0].borrow().$func_name(&args[1].borrow())?; // Never panics when single-threaded.
            return Ok(Rc::new(RefCell::new(res)));
        }
    }
}
macro_rules! create_operator_assign_function {
    ($func_name:ident,$operator_name:ident)=>{
        fn $func_name(args: &[Rc<RefCell<SimpleDynamicValue>>]) -> anyhow::Result<Rc<RefCell<SimpleDynamicValue>>>  {
            let res=args[0].borrow().$operator_name(&args[1].borrow())?; // Never panics when single-threaded.
            *(args[0].borrow_mut())=res; // Never panics when single-threaded.
            return Ok(args[0].clone());
        }
    }
}
macro_rules! create_simple_compare_function {
    ($func_name:ident)=>{
        fn $func_name(args: &[Rc<RefCell<SimpleDynamicValue>>]) -> anyhow::Result<Rc<RefCell<SimpleDynamicValue>>>  {
            let res=args[0].borrow().$func_name(&args[1].borrow())?; // Never panics when single-threaded.
            return Ok(Rc::new(RefCell::new(SimpleDynamicValue::Bool(res))));
        }
    }
}

#[derive(Clone,Debug)]
pub(crate) enum SimpleDynamicValue {
    Unit,
    Bool(bool),
    Integer(rhai_bytecode::INT),
    Float(rhai_bytecode::FLOAT),
    Array(rhai_bytecode::VEC<Rc<RefCell<SimpleDynamicValue>>>),
    Range(rhai_bytecode::INT,rhai_bytecode::INT),
}

impl DynamicValue for SimpleDynamicValue {
    fn from_constant(v:DynamicConstant) -> anyhow::Result<Self> {
        match v {
            DynamicConstant::Unit => {
                return Ok(Self::Unit);
            }
            DynamicConstant::Bool(v) => {
                return Ok(Self::Bool(v));
            }
            DynamicConstant::Integer(v) => {
                return Ok(Self::Integer(v));
            }
            DynamicConstant::Float(v) => {
                return Ok(Self::Float(v));
            }
            DynamicConstant::Array(ary) => {
                let mut new_ary = rhai_bytecode::VEC::<Rc<RefCell<Self>>>::with_capacity(ary.len());
                for v in ary.iter() {
                    new_ary.push(Rc::new(RefCell::new(Self::from_constant(v.clone())?)));
                }
                return Ok(Self::Array(new_ary));
            }
            DynamicConstant::Range(start, len) => {
                return Ok(Self::Range(start, len));
            }
            _=>{
                anyhow::bail!("Connot convert from dynamic constant \"{:?}\"! Unsupported type!", v);
            }
        }
    }
    fn from_unit() -> anyhow::Result<Self> {
        return Ok(Self::Unit);
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
    fn from_char(v:char) -> anyhow::Result<Self> {
        anyhow::bail!("Connot convert from char \"{}\"! Unsupported type!", v);
    }
    fn from_string(v:String) -> anyhow::Result<Self> {
        anyhow::bail!("Connot convert from string \"{}\"! Unsupported type!", v);
    }
    fn from_array(v:rhai_bytecode::VEC<Rc<RefCell<Self>>>) -> anyhow::Result<Self> {
        return Ok(Self::Array(v));
    }
    fn is_unit(&self) -> bool {
        match self {
            Self::Unit => {return true;}
            _ => {return false;}
        }
    }
    fn to_bool(&self) -> anyhow::Result<bool> {
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
    fn to_size(&self) -> anyhow::Result<rhai_bytecode::SIZE> {
        match self {
            Self::Integer(v) => {
                return Ok(*v as rhai_bytecode::SIZE);
            }
            _ => {
                anyhow::bail!("Cannot convert \"{:?}\" to size!", self);
            }
        }
    }
    fn index_into(&self,ind:rhai_bytecode::SIZE)->anyhow::Result<Rc<RefCell<Self>>> {
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
    fn iter(&self,index:rhai_bytecode::SIZE) -> anyhow::Result<Option<Rc<RefCell<Self>>>> {
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
                    return Ok(Some(Rc::new(RefCell::new(Self::Integer(*start+offset)))));
                }
            }
            _=> {
                anyhow::bail!("Cannot iterate over \"{:?}\"!",self);
            }
        }
    }
}

impl SimpleDynamicValue {
    fn not(&self) -> anyhow::Result<Self> {
        match self {
            Self::Unit => {
                return Ok(Self::Bool(true));
            }
            Self::Bool(v) => {
                return Ok(Self::Bool(!*v));
            }
            Self::Integer(v) => {
                return Ok(Self::Bool(*v == 0));
            }
            Self::Float(v) => {
                return Ok(Self::Bool(v.is_nan()||*v == 0.0));
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
            Self::Integer(va) => {
                return Ok(Self::Integer(-va));
            }
            Self::Float(va) => {
                return Ok(Self::Float(-va));
            }
            _=>{
                anyhow::bail!(
                    "Operator \"-\" can not be applied to \"{:?}\"!",
                    self
                );
            }
        }
    }
    create_simple_arithmetic!(add,add_int_int,+);
    create_simple_arithmetic!(subtract,subtract_int_int,-);
    create_simple_arithmetic!(multiply,multiply_int_int,*);
    create_simple_arithmetic!(divide,divide_int_int,/);
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
    create_simple_compare!(equals,==);
    create_simple_compare!(not_equals,!=);
    create_simple_compare!(less_than,<);
    create_simple_compare!(greater_than,>);
    create_simple_compare!(less_than_equal_to,<=);
    create_simple_compare!(greater_than_equal_to,>=);
}

fn not(args: &[Rc<RefCell<SimpleDynamicValue>>]) -> anyhow::Result<Rc<RefCell<SimpleDynamicValue>>>  {
    return Ok(Rc::new(RefCell::new(args[0].borrow().not()?))); // Never panics when single-threaded.
}
create_simple_binary_function!(add);
fn subtract(args: &[Rc<RefCell<SimpleDynamicValue>>]) -> anyhow::Result<Rc<RefCell<SimpleDynamicValue>>>  {
    let res=if args.len() == 1 { // Negative.
        args[0].borrow().negative()? // Never panics when single-threaded.
    }else{
        args[0].borrow().subtract(&args[1].borrow())? // Never panics when single-threaded.
    };
    return Ok(Rc::new(RefCell::new(res)));
}
create_simple_binary_function!(multiply);
create_simple_binary_function!(divide);
create_simple_binary_function!(modulus);
create_simple_binary_function!(power);
fn assign(args: &[Rc<RefCell<SimpleDynamicValue>>]) -> anyhow::Result<Rc<RefCell<SimpleDynamicValue>>>  {
    let rhs=args[1].borrow().clone(); // Never panics when single-threaded.
    *(args[0].borrow_mut())=rhs; // Never panics when single-threaded.
    return Ok(args[0].clone());
}
create_operator_assign_function!(add_assign,add);
create_operator_assign_function!(subtract_assign,subtract);
create_operator_assign_function!(multiply_assign,multiply);
create_operator_assign_function!(divide_assign,divide);
create_simple_compare_function!(equals);
create_simple_compare_function!(not_equals);
create_simple_compare_function!(less_than);
create_simple_compare_function!(greater_than);
create_simple_compare_function!(less_than_equal_to);
create_simple_compare_function!(greater_than_equal_to);
fn range(args: &[Rc<RefCell<SimpleDynamicValue>>]) -> anyhow::Result<Rc<RefCell<SimpleDynamicValue>>>  {
    let a=args[0].borrow(); // Never panics when single-threaded.
    let b=args[1].borrow(); // Never panics when single-threaded.
    match (&*a,&*b) {
        (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
            let l = vb-va;
            if l < 0{
                anyhow::bail!("Range start \"{}\" is greater than end \"{}\"!", va, vb);
            } else{
                return Ok(Rc::new(RefCell::new(SimpleDynamicValue::Range(*va,l))));
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
fn range_inclusive(args: &[Rc<RefCell<SimpleDynamicValue>>]) -> anyhow::Result<Rc<RefCell<SimpleDynamicValue>>>  {
    let a=args[0].borrow(); // Never panics when single-threaded.
    let b=args[1].borrow(); // Never panics when single-threaded.
    match (&*a,&*b) {
        (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
            let l = vb-va;
            if l < 0{
                anyhow::bail!("Range start \"{}\" is greater than end \"{}\"!", va, vb);
            } else{
                return Ok(Rc::new(RefCell::new(SimpleDynamicValue::Range(*va,l+1))));
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

pub(crate) fn new_executer() -> anyhow::Result<rhai_bytecode::Executer<SimpleDynamicValue>> {
    let mut executer = rhai_bytecode::Executer::<SimpleDynamicValue>::new();
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
    executer.add_fn("<=", less_than_equal_to,2,2)?;
    executer.add_fn(">=", greater_than_equal_to,2,2)?;
    executer.add_fn("..", range,2,2)?;
    executer.add_fn("..=", range_inclusive,2,2)?;
    return Ok(executer);
}