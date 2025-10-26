# nanosat-rs

**nanosat-rs** is a minimal [CDCL](https://en.wikipedia.org/wiki/Conflict-driven_clause_learning) [SAT solver](https://en.wikipedia.org/wiki/Boolean_satisfiability_problem) for propositional logic formulas in [CNF](https://en.wikipedia.org/wiki/Conjunctive_normal_form). An example for a SAT instance in CNF is $(A \lor \lnot B) \land (\lnot A \lor B) \land A$ with the model $A = \mathrm{true}$ and $B = \mathrm{true}$.

Inspired by projects like [nanoGPT](https://github.com/karpathy/nanoGPT) and [minisat](https://github.com/niklasso/minisat), `nanosat-rs` aims to provide an educational, readable, and modern implementation of a [Conflict-Driven Clause Learning](https://en.wikipedia.org/wiki/Conflict-driven_clause_learning) solver in pure Rust.

A good conceptual starting point for understanding [CDCL](https://en.wikipedia.org/wiki/Conflict-driven_clause_learning) solvers can be found in [these lecture slides](https://satlecture.github.io/kit2024/) from a course at the Karlsruhe Institute of Technology (KIT).

We have adapted small, relevant pieces from [minisat](https://github.com/niklasso/minisat) where appropriate.

## Comparison

| | nanosat-rs (ours) | [minisat-core](https://github.com/niklasso/minisat) | [kissat](https://github.com/arminbiere/kissat) |
|-|-|-|-|
| Lines of code (`cloc`) | 1,162 | 2,543 | 35,348 |
| Language | Rust | C++ | C |
| No compiler warnings | ✅ | ❌ | ❌ |
| No custom allocators & union magic | ✅ | ❌ | ❌ |
| Speed | 🐢 slow | 🐢 slow | 🐇 fast |

## Example

Running `nanosat-rs` on a problem instance with about 276,000 clauses requires about a minute.

```sh
cargo run --release -- tests/examples/success/hardware_verification.cnf.xz
```

An excerpt from the output of this command:

```txt
...

============================[      Summary      ]==============================
|                                                                             |
|  #Restarts:                     511                                         |
|  #Conflicts:                 224475 (    4274.776/sec)                      |
|  #Decisions:                4923438                                         |
|  #Propagations:           441250019 ( 8402917.146/sec)                      |
|  Total time:              52.511528                                         |
|                                                                             |
===============================================================================

SAT -1 2 3 4 -5 -6 -7 ...
```

## Testing

To build and run all tests

```sh
cargo test
```
