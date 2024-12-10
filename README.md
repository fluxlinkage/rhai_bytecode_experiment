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
- Execution speed is usually faster than using AST (See [Benchmarks](#Benchmarks) below).

## Disadvantages of using bytecode

- All functions and operators in Rhai interpreter cannot be uesd directly, and need to be implemented manually.

## Known Issues

- Experiment Only! i.e. it is unstable and has bugs! Do not use this in production!
- Many features (switch, try, etc) of Rhai not supported yet!

## Benchmarks

The following benchmarks were run on a 4.7GHz Linux ( Debian 12 amd64 ) VM comparing bytecodes with Rhai ASTs.

Benchmarks were performed on Dec 10, 2024. Newer versions may have different results.

| Indicators | [1M Loop](scripts/speed_test.rhai) | [Prime numbers](scripts/prime.rhai) |
| :---: | :---: | :---: |
| Original script size (bytes) | 133 (100%) | 476 (100%) |
| Compressed script size (bytes) | 127 (95%) | 271 (56%) |
| Bytecode JSON size (bytes) | 118 (88%) | 560 (117%) |
| Compressed bytecode JSON size (bytes) | 89 (66%) | 212 (44%) |
| Median execution time for bytecode (s) | 0.048 (62%) | 0.292 (49%) |
| Median execution time for AST (s) | 0.077 (100%) | 0.591 (100%) |
