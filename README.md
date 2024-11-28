# rhai_bytecode_experiment
Simple experiment on [Rhai](https://rhai.rs) bytecode compilation and evaluation.

## Usage

```bash
cargo run --release
```

This will compile the provided Rhai script test, then run it.

## Advantages of using bytecode

- Serialization/deserialization supported.
- Orignal script is not exposed, useful when you want to protect your source code.

## Disadvantages of using bytecode

- All functions and operators in Rhai interpreter cannot be uesd directly, and need to be implemented manually.
- Larger size needed for storage.
- Execution speed is probably NOT faster than using AST.

## Known Issues

- Experiment Only! i.e. it is unstable and has bugs! Do not use this in production!
- Many features (for, switch, try, etc) of Rhai not supported yet!