use rhai::{Engine, EvalAltResult};
use rhai_bytecode::{self, DynamicValue};

#[derive(Clone, Debug)]
enum SimpleDynamicValue {
    Unit,
    Bool(bool),
    Integer(rhai::INT),
    Float(rhai::FLOAT),
    VariableRef(rhai_bytecode::OpSize, Vec<rhai_bytecode::OpSize>),
}

impl rhai_bytecode::DynamicValue for SimpleDynamicValue {
    fn from_dynamic(dynamic: rhai::Dynamic) -> anyhow::Result<Self> {
        // If Dynamic.0 is pub instead of pub(create) with "internal", these could be easier to implement through match.
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
        }
        anyhow::bail!("Unsupported type: {}", dynamic.type_name());
    }

    fn from_unit() -> anyhow::Result<Self> {
        return Ok(SimpleDynamicValue::Unit);
    }

    fn from_bool(v: bool) -> anyhow::Result<Self> {
        return Ok(SimpleDynamicValue::Bool(v));
    }

    fn from_char(v: char) -> anyhow::Result<Self> {
        // Simplely, we'll just cast the char as an integer.
        return Ok(SimpleDynamicValue::Integer(v as rhai::INT));
    }

    fn from_integer(v: rhai::INT) -> anyhow::Result<Self> {
        return Ok(SimpleDynamicValue::Integer(v));
    }

    fn from_float(v: rhai::FLOAT) -> anyhow::Result<Self> {
        return Ok(SimpleDynamicValue::Float(v));
    }

    fn from_variable_ref(var_id: rhai_bytecode::OpSize) -> anyhow::Result<Self> {
        return Ok(SimpleDynamicValue::VariableRef(var_id, vec![]));
    }

    fn from_variable_element_ref(
        var_id: rhai_bytecode::OpSize,
        indexes: Vec<rhai_bytecode::OpSize>,
    ) -> anyhow::Result<Self> {
        return Ok(SimpleDynamicValue::VariableRef(var_id, indexes));
    }

    fn is_unit(&self, variables: &Vec<Self>) -> anyhow::Result<bool> {
        match self.deref(variables)? {
            SimpleDynamicValue::Unit => {
                return Ok(true);
            }
            _ => {
                return Ok(false);
            }
        }
    }

    fn to_bool(&self, variables: &Vec<Self>) -> anyhow::Result<bool> {
        match self.deref(variables)? {
            SimpleDynamicValue::Bool(v) => {
                return Ok(*v);
            }
            SimpleDynamicValue::Integer(v) => {
                return Ok(*v != 0);
            }
            SimpleDynamicValue::Float(v) => {
                return Ok(!v.is_nan() && *v != 0.0);
            }
            _ => {
                anyhow::bail!("Cannot convert \"{:?}\" to bool!", self);
            }
        }
    }

    fn to_size(&self, variables: &Vec<Self>) -> anyhow::Result<rhai_bytecode::OpSize> {
        match self.deref(variables)? {
            SimpleDynamicValue::Integer(v) => {
                return Ok(*v as rhai_bytecode::OpSize);
            }
            _ => {
                anyhow::bail!("Cannot convert \"{:?}\" to size!", self);
            }
        }
    }

    fn deref<'a>(&'a self, variables: &'a Vec<Self>) -> anyhow::Result<&'a Self> {
        match self {
            SimpleDynamicValue::VariableRef(var_id, indexes) => {
                if indexes.is_empty() {
                    return variables[*var_id as usize].deref(variables);
                } else {
                    anyhow::bail!("Element reference not implemented yet!");
                }
            }
            _ => {
                return Ok(self);
            }
        }
    }

    fn deref_mut<'a>(&self, variables: &'a mut Vec<Self>) -> anyhow::Result<&'a mut Self> {
        match self {
            SimpleDynamicValue::VariableRef(var_id, indexes) => {
                if indexes.is_empty() {
                    match variables[*var_id as usize] {
                        SimpleDynamicValue::VariableRef(..) => {
                            anyhow::bail!(
                                "Cascade variable mutable references are not implemented yet!"
                            );
                        }
                        _ => {
                            return Ok(&mut variables[*var_id as usize]);
                        }
                    }
                } else {
                    anyhow::bail!("Element reference not implemented yet!");
                }
            }
            _ => {
                anyhow::bail!("Variable \"{:?}\" is not a reference!", self);
            }
        }
    }

    fn ref_append_index(&mut self, ind: rhai_bytecode::OpSize) -> anyhow::Result<()> {
        match self {
            SimpleDynamicValue::VariableRef(var_id, indexes) => {
                indexes.push(ind);
                return Ok(());
            }
            _ => {
                anyhow::bail!("Variable \"{:?}\" is not a reference!", self);
            }
        }
    }
}

fn greater_than(
    args: Vec<SimpleDynamicValue>,
    variables: &mut Vec<SimpleDynamicValue>,
) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \">\" needs 2 arguments, but {} provided!",
            args.len()
        );
    } else {
        let a = args[0].deref(variables)?;
        let b = args[1].deref(variables)?;
        match (a, b) {
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(*va > *vb));
            }
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool(*va as rhai::FLOAT > *vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                return Ok(SimpleDynamicValue::Bool(*va > *vb));
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                return Ok(SimpleDynamicValue::Bool(*va > *vb as rhai::FLOAT));
            }
            _ => {
                anyhow::bail!(
                    "Operator \">\" can only be applied to \"{:?}\" and \"{:?}\"!",
                    a,
                    b
                );
            }
        }
    }
}

fn minus_assign(
    args: Vec<SimpleDynamicValue>,
    variables: &mut Vec<SimpleDynamicValue>,
) -> anyhow::Result<SimpleDynamicValue> {
    if args.len() != 2 {
        anyhow::bail!(
            "Operator \"-=\" needs 2 arguments, but {} provided!",
            args.len()
        );
    } else {
        let a = args[0].deref(variables)?;
        let b = args[1].deref(variables)?;
        let res = match (a, b) {
            (SimpleDynamicValue::Integer(va), SimpleDynamicValue::Integer(vb)) => {
                SimpleDynamicValue::Integer(*va - *vb)
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Float(vb)) => {
                SimpleDynamicValue::Float(*va - *vb)
            }
            (SimpleDynamicValue::Float(va), SimpleDynamicValue::Integer(vb)) => {
                SimpleDynamicValue::Float(*va - *vb as rhai::FLOAT)
            }
            _ => {
                anyhow::bail!(
                    "Operator \"-=\" can only be applied to \"{:?}\" and \"{:?}\"!",
                    a,
                    b
                );
            }
        };
        let a_mut = args[0].deref_mut(variables)?;
        *a_mut = res.clone();
        return Ok(res);
    }
}

fn main() -> Result<(), Box<EvalAltResult>> {
    let script = "let x = 1_000_000;
while x > 0 {
x -= 1;
}";
    let engine = Engine::new();
    let ast = engine.compile(script)?;
    let mut executer = rhai_bytecode::Executer::<SimpleDynamicValue>::new();
    executer
        .add_fn(">".to_string(), Box::new(greater_than))
        .unwrap();
    executer
        .add_fn("-=".to_string(), Box::new(minus_assign))
        .unwrap();
    let mut variable_names = Vec::<String>::new();
    let byte_codes =
        rhai_bytecode::ast_to_byte_codes(&executer, &mut variable_names, &ast).unwrap();
    let json = serde_json::to_string(&byte_codes).unwrap();
    println!("Serilized json = {}", json);
    let byte_codes_restored = serde_json::from_str::<Vec<rhai_bytecode::ByteCode>>(&json).unwrap();
    let now = std::time::Instant::now();
    rhai_bytecode::run_byte_codes(
        &executer,
        &byte_codes_restored,
        variable_names.len(),
        &vec![],
    )
    .unwrap();
    println!(
        "Finished (ByteCode). Run time = {} seconds.",
        now.elapsed().as_secs_f64()
    );
    let now = std::time::Instant::now();
    engine.run_ast(&ast)?;
    println!(
        "Finished (AST). Run time = {} seconds.",
        now.elapsed().as_secs_f64()
    );
    return Ok(());
}
