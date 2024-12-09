use std::cell::RefCell;
use std::rc::Rc;
pub use rhai;
use rhai::{Expr, Stmt};

thread_local! {
    static COMPILE_ENGINE: std::cell::RefCell<rhai::Engine> = std::cell::RefCell::new(rhai::Engine::new_raw());
}

#[cfg(feature = "thin-vec")]
#[macro_use] extern crate thin_vec;

#[cfg(feature = "size16")]
pub type SIZE = u16;
#[cfg(feature = "size32")]
pub type SIZE = u32;
#[cfg(feature = "size64")]
pub type SIZE = u64;

pub type INT = rhai::INT;
pub type FLOAT = rhai::FLOAT;
#[cfg(feature = "thin-vec")]
pub type VEC<T> = thin_vec::ThinVec<T>;
#[cfg(not(feature = "thin-vec"))]
pub type VEC<T> = Vec<T>;

pub fn new_vec<T:Clone>(element:T,size:usize) -> VEC<T> {
    #[cfg(feature = "thin-vec")]
    return thin_vec![element;size];
    #[cfg(not(feature = "thin-vec"))]
    return vec![element;size];
}

#[derive(Clone,Debug, serde::Serialize, serde::Deserialize)]
pub enum DynamicConstant {
    #[serde(rename="U")]
    Unit,
    #[serde(rename="B")]
    Bool(bool),
    #[serde(rename="I")]
    Integer(INT),
    #[serde(rename="F")]
    Float(FLOAT),
    #[serde(rename="C")]
    Char(char),
    #[serde(rename="S")]
    String(String),
    #[serde(rename="A")]
    Array(VEC<DynamicConstant>),
    #[serde(rename="R")]
    Range(INT,INT),
}

impl DynamicConstant{
    fn from_dynamic(dynamic: &rhai::Dynamic) -> anyhow::Result<Self> {
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
        } else if dynamic.is_char() {
            match dynamic.as_char() {
                Ok(v) => {
                    return Ok(Self::Char(v));
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert rhai::Dynamic to char!");
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
        } else if dynamic.is_string() {
           return Ok(Self::String(dynamic.to_string()));
        }else if dynamic.is_array() {
            match dynamic.as_array_ref() {
                Ok(ary) => {
                    let mut vec=VEC::with_capacity(ary.len());
                    for item in ary.iter() {
                        vec.push(Self::from_dynamic(item)?);
                    }
                    return Ok(Self::Array(vec));
                }
                Err(_) => {
                    anyhow::bail!("Failed to convert rhai::Dynamic to array!");
                }
            }
        }else if dynamic.type_id()== std::any::TypeId::of::<std::ops::Range<INT>>() {
            match dynamic.clone().try_cast_result::<std::ops::Range<INT>>() {
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
        }else if dynamic.type_id()== std::any::TypeId::of::<std::ops::RangeInclusive<INT>>() {
            match dynamic.clone().try_cast_result::<std::ops::RangeInclusive<INT>>() {
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
}

pub trait DynamicValue: Sized + Clone {
    fn from_constant(v:DynamicConstant) -> anyhow::Result<Self>;
    fn from_unit() -> anyhow::Result<Self>;
    fn from_bool(v:bool) -> anyhow::Result<Self>;
    fn from_integer(v:INT) -> anyhow::Result<Self>;
    fn from_float(v:FLOAT) -> anyhow::Result<Self>;
    fn from_char(v:char) -> anyhow::Result<Self>;
    fn from_string(v:String) -> anyhow::Result<Self>;
    fn from_array(v:VEC<Rc<RefCell<Self>>>) -> anyhow::Result<Self>;
    fn is_unit(&self) -> bool;
    fn to_bool(&self) -> anyhow::Result<bool>;
    fn to_size(&self) -> anyhow::Result<SIZE>;
    fn index_into(&self,ind:SIZE)->anyhow::Result<Rc<RefCell<Self>>>;
    fn iter(&self,index:SIZE) -> anyhow::Result<Option<Rc<RefCell<Self>>>>;
}

#[derive(Clone,Debug,serde::Serialize, serde::Deserialize)]
pub enum ByteCode {
    #[serde(rename="DC")]
    DynamicConstant(DynamicConstant),
    #[serde(rename="UC")]
    UnitConstant,
    #[serde(rename="BC")]
    BoolConstant(bool),
    #[serde(rename="IC")]
    IntegerConstant(INT),
    #[serde(rename="FC")]
    FloatConstant(FLOAT),
    #[serde(rename="CC")]
    CharConstant(char),
    #[serde(rename="SC")]
    StringConstant(String),
    #[serde(rename="IS")]
    InterpolatedString(SIZE),
    #[serde(rename="CA")]
    ConstructArray(SIZE),
    #[serde(rename="V")]
    Variable(SIZE),
    #[serde(rename="F")]
    FnCall(SIZE, SIZE),
    #[serde(rename="J")]
    Jump(SIZE),
    #[serde(rename="JT")]
    JumpIfTrue(SIZE),
    #[serde(rename="JF")]
    JumpIfFalse(SIZE),
    #[serde(rename="JNN")]
    JumpIfNotNull(SIZE),
    #[serde(rename="VI")]
    VarInit(SIZE),
    #[serde(rename="I")]
    Index,
    #[serde(rename="IT")]
    Iter(SIZE,SIZE,SIZE,SIZE),
    #[serde(rename="R")]
    Return,
    #[serde(rename="P")]
    PopStack,
}

pub struct Executer<B: DynamicValue+std::fmt::Debug> {
    fn_names: Vec<String>,
    fns: Vec<Box<dyn Fn(&[Rc<RefCell<B>>]) -> anyhow::Result<Rc<RefCell<B>>>>>,
    fn_arg_ranges: Vec<(SIZE,SIZE)>,
}

impl<B: DynamicValue+std::fmt::Debug> Executer<B> {
    pub fn new() -> Self {
        return Self {
            fn_names: vec![],
            fns: vec![],
            fn_arg_ranges: vec![],
        };
    }
    fn function_names(&self) -> &Vec<String> {
        return &self.fn_names;
    }
    pub fn add_fn<F:Fn(&[Rc<RefCell<B>>]) -> anyhow::Result<Rc<RefCell<B>>>+'static>(
        &mut self,
        name: impl ToString,
        func: F,
        min_args: SIZE,
        max_args: SIZE,
    ) -> anyhow::Result<()> {
        let name_string = name.to_string();
        if self.fn_names.contains(&name_string) {
            anyhow::bail!("Function \"{}\" already exists!", name_string);
        } else {
            if min_args > max_args {
                anyhow::bail!(
                    "Minimum arguments for function \"{}\" is greater than maximum!",
                    name_string
                );
            }
            self.fns.push(Box::new(func));
            self.fn_arg_ranges.push((min_args, max_args));
            self.fn_names.push(name_string);
            return Ok(());
        }
    }
    fn check_fn_arg_count(&self, index: SIZE, arg_count: SIZE) -> anyhow::Result<()> {
        let ind = index as usize;
        if ind >= self.fns.len() {
            anyhow::bail!("Function \"{}\" does not exist!", self.fn_names[ind]);
        }
        let (min_args, max_args) = &self.fn_arg_ranges[ind];
        if arg_count < *min_args {
            anyhow::bail!("Function \"{}\" requires at least {} arguments, but {} given!",self.fn_names[ind], min_args, arg_count);
        }
        if arg_count > *max_args {
            anyhow::bail!("Function \"{}\" requires at most {} arguments, but {} given!", self.fn_names[ind], max_args, arg_count);
        }
        return  Ok(());
    }
    fn call_fn(&self, index: SIZE, args: &[Rc<RefCell<B>>]) -> anyhow::Result<Rc<RefCell<B>>> {
        let ind = index as usize;
        return self.fns[ind](args);
    }
}

fn find_index(vec: &Vec<String>, name: &str, type_str: &str) -> anyhow::Result<SIZE> {
    match vec.iter().rposition(|x| x == name) {
        Some(i) => {
            return Ok(i as SIZE);
        }
        None => {
            anyhow::bail!("Undefined {} \"{}\"!", type_str, name);
        }
    }
}

fn append_return_index(vec: &mut Vec<String>, name: &str) -> SIZE {
    vec.push(name.to_string());
    return (vec.len() - 1) as SIZE;
}

fn append_expr(
    functions: &Vec<String>,
    variables: &mut Vec<String>,
    max_variable_count: &mut SIZE,
    break_pos: &mut Vec<usize>,
    continue_pos: &mut Vec<usize>,
    byte_codes: &mut Vec<ByteCode>,
    expr: &Expr,
) -> anyhow::Result<()> {
    match expr {
        Expr::DynamicConstant(dynamic, _) => {
            byte_codes.push(ByteCode::DynamicConstant(DynamicConstant::from_dynamic(dynamic)?));
        }
        Expr::BoolConstant(v, _) => {
            byte_codes.push(ByteCode::BoolConstant(*v));
        }
        Expr::IntegerConstant(v, _) => {
            byte_codes.push(ByteCode::IntegerConstant(*v));
        }
        Expr::FloatConstant(float_wrapper, _) => {
            byte_codes.push(ByteCode::FloatConstant(*float_wrapper.as_ref()));
        }
        Expr::CharConstant(v, _) => {
            byte_codes.push(ByteCode::CharConstant(*v));
        }
        Expr::StringConstant(immutable_string, _) => {
            byte_codes.push(ByteCode::StringConstant(immutable_string.to_string()));
        }
        Expr::InterpolatedString(thin_vec, _) => {
            for expr in thin_vec {
                append_expr(
                    functions,
                    variables,
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    expr,
                )?;
            }
            byte_codes.push(ByteCode::InterpolatedString(thin_vec.len() as SIZE));
        }
        Expr::Array(thin_vec, _) => {
            for sub_expr in thin_vec {
                append_expr(
                    functions,
                    variables,
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_expr,
                )?;
            }
            byte_codes.push(ByteCode::ConstructArray(thin_vec.len() as SIZE));
        }
        Expr::Map(..) => {
            anyhow::bail!("Map not supported yet!");
        }
        Expr::Unit(..) => {
            byte_codes.push(ByteCode::UnitConstant);
        }
        Expr::Variable(data, _, _) => {
            let var_id = find_index(variables, data.1.as_str(), "variable")?;
            byte_codes.push(ByteCode::Variable(var_id));
        }
        Expr::ThisPtr(..) => {
            anyhow::bail!("\"this\" pointer not supported yet!");
        }
        Expr::Property(..) => {
            anyhow::bail!("Property not supported yet!");
        }
        Expr::MethodCall(..) => {
            anyhow::bail!("Method not supported yet!");
        }
        Expr::Stmt(stmt_block) => {
            for stmt in stmt_block.iter() {
                append_stmt(
                    functions,
                    variables,
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    stmt,
                )?;
            }
        }
        Expr::FnCall(fn_call_expr, _) => {
            for sub_expr in &fn_call_expr.args {
                append_expr(
                    functions,
                    variables,
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_expr,
                )?;
            }
            let fn_id = find_index(functions, fn_call_expr.name.as_str(), "function")?;
            byte_codes.push(ByteCode::FnCall(fn_id, fn_call_expr.args.len() as SIZE));
        }
        Expr::Dot(..) => {
            anyhow::bail!("Dot operator (.) not supported yet!");
        }
        Expr::Index(binary_expr, astflags, _) => {
            if (*astflags & rhai::ASTFlags::NEGATED) == rhai::ASTFlags::NEGATED {
                anyhow::bail!("Operator (?[]) not supported yet!");
            }
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.lhs,
            )?;
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.rhs,
            )?;
            byte_codes.push(ByteCode::Index);
        }
        Expr::And(binary_expr, _) => {
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.lhs,
            )?;
            let pos = byte_codes.len();
            byte_codes.push(ByteCode::JumpIfFalse(0));
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.rhs,
            )?;
            byte_codes[pos] = ByteCode::JumpIfFalse(byte_codes.len() as SIZE);
        }
        Expr::Or(binary_expr, _) => {
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.lhs,
            )?;
            let pos = byte_codes.len();
            byte_codes.push(ByteCode::JumpIfTrue(0));
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.rhs,
            )?;
            byte_codes[pos] = ByteCode::JumpIfTrue(byte_codes.len() as SIZE);
        }
        Expr::Coalesce(binary_expr, _) => {
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.lhs,
            )?;
            let pos = byte_codes.len();
            byte_codes.push(ByteCode::JumpIfNotNull(0));
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.rhs,
            )?;
            byte_codes[pos] = ByteCode::JumpIfNotNull(byte_codes.len() as SIZE);
        }
        // Expr::Custom(..) => {
        //     anyhow::bail!("Custom syntax not supported yet!");
        // }
        _ => {
            anyhow::bail!("Unknown expression type for \"{:?}\"!", expr);
        }
    }
    return Ok(());
}

fn append_stmt(
    functions: &Vec<String>,
    variables: &mut Vec<String>,
    max_variable_count: &mut SIZE,
    break_pos: &mut Vec<usize>,
    continue_pos: &mut Vec<usize>,
    byte_codes: &mut Vec<ByteCode>,
    stmt: &Stmt,
) -> anyhow::Result<()> {
    match stmt {
        Stmt::Noop(_) => {
            //Do nothing.
        }
        Stmt::If(flow_control, _) => {
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &flow_control.expr,
            )?;
            let jz_pos = byte_codes.len();
            byte_codes.push(ByteCode::JumpIfFalse(0));
            let var_len=variables.len();
            for sub_stmt in &flow_control.body {
                append_stmt(
                    functions,
                    variables,
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_stmt,
                )?;
            }
            variables.truncate(var_len);
            if flow_control.branch.is_empty() {
                byte_codes[jz_pos] = ByteCode::JumpIfFalse(byte_codes.len() as SIZE);
            }else{
                let jmp_pos = byte_codes.len();
                byte_codes.push(ByteCode::Jump(0));
                byte_codes[jz_pos] = ByteCode::JumpIfFalse(byte_codes.len() as SIZE);
                let var_len=variables.len();
                for sub_stmt in &flow_control.branch {
                    append_stmt(
                        functions,
                        variables,
                        max_variable_count,
                        break_pos,
                        continue_pos,
                        byte_codes,
                        sub_stmt,
                    )?;
                }
                variables.truncate(var_len);
                byte_codes[jmp_pos] = ByteCode::Jump(byte_codes.len() as SIZE);
            }
        }
        Stmt::Switch(..) => {
            anyhow::bail!("\"switch\" not supported yet!");
        }
        Stmt::While(flow_control, _) => {
            let start_pos = byte_codes.len();
            let jz_pos = match flow_control.expr {
                Expr::Unit(_) => usize::MAX,
                _ => {
                    append_expr(
                        functions,
                        variables,
                        max_variable_count,
                        break_pos,
                        continue_pos,
                        byte_codes,
                        &flow_control.expr,
                    )?;
                    byte_codes.push(ByteCode::JumpIfFalse(0));
                    byte_codes.len() - 1
                }
            };
            let mut new_break_pos = Vec::<usize>::new();
            let mut new_continue_pos = Vec::<usize>::new();
            let var_len=variables.len();
            for sub_stmt in &flow_control.body {
                append_stmt(
                    functions,
                    variables,
                    max_variable_count,
                    &mut new_break_pos,
                    &mut new_continue_pos,
                    byte_codes,
                    sub_stmt,
                )?;
            }
            variables.truncate(var_len);
            byte_codes.push(ByteCode::Jump(start_pos as SIZE));
            let end_pos = byte_codes.len();
            for pos_break in &new_break_pos {
                byte_codes[*pos_break] = ByteCode::Jump(end_pos as SIZE);
            }
            for pos_continue in &new_continue_pos {
                byte_codes[*pos_continue] = ByteCode::Jump(start_pos as SIZE);
            }
            if jz_pos != usize::MAX {
                byte_codes[jz_pos] = ByteCode::JumpIfFalse(end_pos as SIZE);
            }
        }
        Stmt::Do(..) => {
            anyhow::bail!("\"do\" not supported yet!");
        }
        Stmt::For(data, _) => {
            let loop_var_id = append_return_index(variables, data.0.as_str());
            let loop_index_id = append_return_index(variables, match &data.1 {
                Some(name) => name.as_str(),
                None => "(loop_index)"
            });
            let loop_range_id = append_return_index(variables, "(loop_range)");
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &data.2.expr,
            )?;
            byte_codes.push(ByteCode::VarInit(loop_range_id));
            byte_codes.push(ByteCode::PopStack);
            byte_codes.push(ByteCode::IntegerConstant(0));
            byte_codes.push(ByteCode::VarInit(loop_index_id));
            byte_codes.push(ByteCode::PopStack);
            let start_pos = byte_codes.len();
            // byte_codes.push(ByteCode::Variable(loop_range_id));
            // byte_codes.push(ByteCode::Variable(loop_index_id));
            // byte_codes.push(ByteCode::Variable(loop_var_id));
            // let jz_pos = byte_codes.len();
            byte_codes.push(ByteCode::Iter(loop_range_id,loop_index_id,loop_var_id,0));
            let mut new_break_pos = Vec::<usize>::new();
            let mut new_continue_pos = Vec::<usize>::new();
            let var_len=variables.len();
            for sub_stmt in &data.2.body {
                append_stmt(
                    functions,
                    variables,
                    max_variable_count,
                    &mut new_break_pos,
                    &mut new_continue_pos,
                    byte_codes,
                    sub_stmt,
                )?;
            }
            variables.truncate(var_len);
            byte_codes.push(ByteCode::Jump(start_pos as SIZE));
            let end_pos = byte_codes.len();
            for pos_break in &new_break_pos {
                byte_codes[*pos_break] = ByteCode::Jump(end_pos as SIZE);
            }
            for pos_continue in &new_continue_pos {
                byte_codes[*pos_continue] = ByteCode::Jump(start_pos as SIZE);
            }
            byte_codes[start_pos] = ByteCode::Iter(loop_range_id,loop_index_id,loop_var_id,end_pos as SIZE);
        }
        Stmt::Var(data, _, _) => {
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &data.1,
            )?;
            let var_id = append_return_index(variables, data.0.as_str());
            if var_id+1>*max_variable_count {
                *max_variable_count=var_id+1;
            }
            byte_codes.push(ByteCode::VarInit(var_id));
            byte_codes.push(ByteCode::PopStack);
        }
        Stmt::Assignment(data) => {
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &data.1.lhs,
            )?;
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &data.1.rhs,
            )?;
            let op_str = match data.0.get_op_assignment_info() {
                Some(info) => info.3,
                None => "=",
            };
            let op_id = find_index(functions, op_str, "assignment operator")?;
            byte_codes.push(ByteCode::FnCall(op_id, 2));
            byte_codes.push(ByteCode::PopStack);
        }
        Stmt::FnCall(fn_call_expr, _) => {
            for sub_expr in &fn_call_expr.args {
                append_expr(
                    functions,
                    variables,
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_expr,
                )?;
            }
            let fn_id = find_index(functions, fn_call_expr.name.as_str(), "function")?;
            byte_codes.push(ByteCode::FnCall(fn_id, fn_call_expr.args.len() as SIZE));
            byte_codes.push(ByteCode::PopStack);
        }
        Stmt::Block(stmt_block) => {
            let var_len=variables.len();
            for stmt in stmt_block.iter() {
                append_stmt(
                    functions,
                    variables,
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    stmt,
                )?;
            }
            variables.truncate(var_len);
        }
        Stmt::TryCatch(..) => {
            anyhow::bail!("\"try\" not supported yet!");
        }
        Stmt::Expr(expr) => {
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                expr,
            )?;
        }
        Stmt::BreakLoop(_, astflags, _) => {
            if (*astflags & rhai::ASTFlags::BREAK) == rhai::ASTFlags::BREAK {
                //break
                break_pos.push(byte_codes.len());
            } else {
                //continue
                continue_pos.push(byte_codes.len());
            }
            byte_codes.push(ByteCode::Jump(0));
        }
        Stmt::Return(expr, astflags, _) => {
            if (*astflags & rhai::ASTFlags::BREAK) == rhai::ASTFlags::BREAK {
                anyhow::bail!("\"throw\" not supported yet!");
            } else {
                match expr {
                    Some(exp) => {
                        append_expr(
                            functions,
                            variables,
                            max_variable_count,
                            break_pos,
                            continue_pos,
                            byte_codes,
                            exp,
                        )?;
                    }
                    None => {
                        byte_codes.push(ByteCode::UnitConstant);
                    }
                }
                byte_codes.push(ByteCode::Return);
            }
        }
        // Stmt::Import(..) => todo!(),
        // Stmt::Export(..) => todo!(),
        // Stmt::Share(..) => todo!(),
        _ => {
            anyhow::bail!("Unknown statement type for \"{:?}\"!", stmt);
        }
    }
    return Ok(());
}

fn trace_jump(init_pos:SIZE,byte_codes: &Vec<ByteCode>)->SIZE {
    let init_pos_sz= init_pos as usize;
    if init_pos_sz < byte_codes.len() {
        match byte_codes[init_pos_sz] {
            ByteCode::Jump(pos) => {
                return trace_jump(pos, byte_codes);
            }
            _=>{}
        }
    }
    return init_pos;
}

pub fn ast_to_byte_codes<B: DynamicValue+std::fmt::Debug>(
    executer: &Executer<B>,
    initial_variables: &mut Vec<String>,
    ast: &rhai::AST,
) -> anyhow::Result<Vec<ByteCode>> {
    let functions = executer.function_names();
    let mut byte_codes = Vec::<ByteCode>::new();
    let mut break_pos = Vec::<usize>::new();
    let mut continue_pos = Vec::<usize>::new();
    let mut max_variable_count=initial_variables.len() as SIZE;
    for stmt in ast.statements() {
        append_stmt(
            functions,
            initial_variables,
            &mut max_variable_count,
            &mut break_pos,
            &mut continue_pos,
            &mut byte_codes,
            stmt,
        )?;
    }
    if !break_pos.is_empty() || !continue_pos.is_empty() {
        anyhow::bail!("Invalid \"break\" or \"continue\" statements without a loop!");
    }
    match byte_codes.last() {
        Some(code) => match code {
            ByteCode::PopStack => {
                byte_codes.pop();
            }
            _ => {}
        },
        None => {}
    }
    for i in 0..byte_codes.len() {
        match &byte_codes[i] {
            ByteCode::Jump(pos) => {
                byte_codes[i]=ByteCode::Jump(trace_jump(*pos,&byte_codes));
            }
            ByteCode::JumpIfTrue(pos) => {
                byte_codes[i]=ByteCode::JumpIfTrue(trace_jump(*pos,&byte_codes));
            }
            ByteCode::JumpIfFalse(pos) => {
                byte_codes[i]=ByteCode::JumpIfFalse(trace_jump(*pos,&byte_codes));
            }
            ByteCode::JumpIfNotNull(pos) => {
                byte_codes[i]=ByteCode::JumpIfNotNull(trace_jump(*pos,&byte_codes));
            }
            ByteCode::Iter(loop_range_id,loop_index_id,loop_var_id,pos) => {
                byte_codes[i]=ByteCode::Iter(*loop_range_id,*loop_index_id,*loop_var_id,trace_jump(*pos,&byte_codes));
            }
            _=>{}
        }
    }
    return Ok(byte_codes);
}

pub fn script_to_byte_codes<B: DynamicValue+std::fmt::Debug>(
    executer: &Executer<B>,
    initial_variables: &mut Vec<String>,
    script: &str,
) -> anyhow::Result<Vec<ByteCode>,> {
    let ast=COMPILE_ENGINE.with_borrow(|engine|engine.compile(script))?;
    return ast_to_byte_codes(executer, initial_variables, &ast);
}

pub fn script_to_byte_codes_expression<B: DynamicValue+std::fmt::Debug>(
    executer: &Executer<B>,
    initial_variables: &mut Vec<String>,
    script: &str,
) -> anyhow::Result<Vec<ByteCode>> {
    let ast=COMPILE_ENGINE.with_borrow(|engine|engine.compile_expression(script))?;
    return ast_to_byte_codes(executer, initial_variables, &ast);
}

pub fn script_to_byte_codes_expression_no_new_variables<B: DynamicValue+std::fmt::Debug>(
    executer: &Executer<B>,
    initial_variables: &mut Vec<String>,
    script: &str,
) -> anyhow::Result<Vec<ByteCode>> {
    let ast=COMPILE_ENGINE.with_borrow(|engine|engine.compile_expression(script))?;
    let init_len = initial_variables.len();
    let res = ast_to_byte_codes(executer, initial_variables, &ast)?;
    if initial_variables.len() != init_len {
        initial_variables.truncate(init_len);
        anyhow::bail!("The script should not declare new variables!");
    } else {
        return Ok(res);
    }
}

pub fn run_byte_codes<B:DynamicValue+std::fmt::Debug>(
    executer: &Executer<B>,
    byte_codes: &Vec<ByteCode>,
    init_vars: &Vec<B>,
) -> anyhow::Result<B> {
    let mut max_var_id=0 as SIZE;
    for byte_code in byte_codes {
        match byte_code {
            ByteCode::Variable(var_id) => {
                if *var_id > max_var_id {
                    max_var_id=*var_id;
                }
            }
            ByteCode::FnCall(fn_id, arg_count) => {
                executer.check_fn_arg_count(*fn_id, *arg_count)?;
            }
            _=>{}
        }
    }
    let var_count=max_var_id as usize+1;
    let mut variables=Vec::<Rc<RefCell<B>>>::with_capacity(var_count);
    let init_len=usize::min(var_count, init_vars.len());
    for i in 0..init_len {
        variables.push(Rc::new(RefCell::new(init_vars[i].clone())));
    }
    for _i in init_len..var_count {
        variables.push(Rc::new(RefCell::new(B::from_unit()?)));
    }
    let mut variable_stack = Vec::<Rc<RefCell<B>>>::new();
    let mut pos = 0usize;
    while pos < byte_codes.len() {
        //println!("{}: {:?}", pos, byte_codes[pos]);
        match &byte_codes[pos] {
            ByteCode::DynamicConstant(dynamic) => {
                variable_stack.push(Rc::new(RefCell::new(B::from_constant(dynamic.to_owned())?)));
            }
            ByteCode::UnitConstant => {
                variable_stack.push(Rc::new(RefCell::new(B::from_unit()?)));
            }
            ByteCode::BoolConstant(v) => {
                variable_stack.push(Rc::new(RefCell::new(B::from_bool(*v)?)));
            }
            ByteCode::IntegerConstant(v) => {
                variable_stack.push(Rc::new(RefCell::new(B::from_integer(*v)?)));
            }
            ByteCode::FloatConstant(v) => {
                variable_stack.push(Rc::new(RefCell::new(B::from_float(*v)?)));
            }
            ByteCode::CharConstant(v) => {
                variable_stack.push(Rc::new(RefCell::new(B::from_char(*v)?)));
            }
            ByteCode::StringConstant(v) => {
                variable_stack.push(Rc::new(RefCell::new(B::from_string(v.to_owned())?)));
            }
            ByteCode::InterpolatedString(_) => {
                anyhow::bail!("InterpolatedString not supported yet!");
            }
            ByteCode::ConstructArray(l) => {
                let len=*l as usize;
                if variable_stack.len() < len {
                    anyhow::bail!("Not enough elements to construct array");
                }
                let ary=variable_stack.split_off(variable_stack.len() - len);
                variable_stack.push(Rc::new(RefCell::new(B::from_array(ary)?)));
            }
            ByteCode::Variable(var_id) => {
                variable_stack.push(variables[*var_id as usize].clone());
            }
            ByteCode::FnCall(fn_index, fn_arg_count) => {
                let fn_arg_count_sz = *fn_arg_count as usize;
                if variable_stack.len() < fn_arg_count_sz {
                    anyhow::bail!("Not enough arguments for function call!");
                }
                let start_pos=variable_stack.len() - fn_arg_count_sz;
                let res=executer.call_fn(*fn_index,&variable_stack[start_pos..])?;
                variable_stack.truncate(start_pos);
                variable_stack.push(res);
                // variable_stack.truncate(start_pos+1);
                // variable_stack[start_pos]=res;
            }
            ByteCode::Jump(p) => {
                pos = *p as usize;
                continue;
            }
            ByteCode::JumpIfTrue(p) => match variable_stack.pop() {
                Some(val) => {
                    if val.borrow().to_bool()? { // Never panics when single-threaded.
                        pos = *p as usize;
                        continue;
                    }
                }
                None => {
                    anyhow::bail!("Not enough arguments for conditional jump!");
                }
            },
            ByteCode::JumpIfFalse(p) => match variable_stack.pop() {
                Some(val) => {
                    if !val.borrow().to_bool()? { // Never panics when single-threaded.
                        pos = *p as usize;
                        continue;
                    }
                }
                None => {
                    anyhow::bail!("Not enough arguments for conditional jump!");
                }
            },
            ByteCode::JumpIfNotNull(p) => match variable_stack.pop() {
                Some(val) => {
                    if !val.borrow().is_unit() { // Never panics when single-threaded.
                        pos = *p as usize;
                        continue;
                    }
                }
                None => {
                    anyhow::bail!("Not enough arguments for conditional jump!");
                }
            },
            ByteCode::VarInit(var_id) => match variable_stack.last() {
                Some(val) => {
                    variables[*var_id as usize]=val.clone();
                }
                None => {
                    anyhow::bail!("Not enough arguments for variable declare!");
                }
            },
            ByteCode::Index => match variable_stack.pop() {
                Some(ind) => match variable_stack.last_mut() {
                    Some(r) => {
                        let index=ind.borrow().to_size()?; // Never panics when single-threaded.
                        let res=r.borrow().index_into(index)?; // Never panics when single-threaded.
                        *r=res;
                    }
                    None => {
                        anyhow::bail!("Not enough arguments for index!");
                    }
                },
                None => {
                    anyhow::bail!("Not enough arguments for index!");
                }
            },
            ByteCode::Return => match variable_stack.pop() {
                Some(value) => {
                    return Ok(value.borrow().to_owned()); // Never panics when single-threaded.
                }
                None => {
                    anyhow::bail!("Missing return value!");
                }
            },
            ByteCode::PopStack => {
                variable_stack.pop();
            }
            ByteCode::Iter(loop_range_id,loop_index_id,loop_var_id,p) => {
                let index=variables[*loop_index_id as usize].borrow().to_size()?; // Never panics when single-threaded.
                let index_res=variables[*loop_range_id as usize].borrow().iter(index)?; // Never panics when single-threaded.
                match index_res {
                    Some(v) => {
                        variables[*loop_var_id as usize]=v;
                        let new_index=index+1;
                        variables[*loop_index_id as usize]=Rc::new(RefCell::new(B::from_integer(new_index as INT)?));
                    }
                    None => {
                        pos = *p as usize;
                        continue;
                    }
                }
            }
        }
        pos += 1;
    }
    //println!("Stack size: {}",variable_stack.len());
    match variable_stack.pop() {
        Some(value) =>{
            return Ok(value.borrow().to_owned()); // Never panics when single-threaded.
        }
        None => {
            return Ok(B::from_unit()?);
        }
    }
}
