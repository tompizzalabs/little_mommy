We are making a Node-based Analog Synthesizer in Rust with nice-plug

## Code Philosophy — Human First

Readability above all. Code is written for humans.

Descriptive naming. Every variable, function, type, and module must have a name with unmistakably clear purpose. No cryptic abbreviations or meaningless names.

No comments. Code must explain itself through good naming, structure, and organization. No comments, TODOs, or commented-out code.

Simplicity first. Always choose the simplest solution that solves the actual problem.

No overengineering. Avoid unnecessary abstractions, wrappers, layers, generics, dependencies, and design patterns.

Explicit over clever. Prefer straightforward logic over tricks, implicit behavior, and overly compact expressions.

Predictable control flow. Avoid deep nesting, hidden side effects, and unnecessary indirection.

Consistency matters. Follow established project conventions. Don't introduce new patterns without a good reason.

No unnecessary duplication. Reuse existing functionality without creating excessive coupling.

Meaningful errors. Handle failures explicitly. Never silently swallow errors.

Performance with common sense. Avoid unnecessary allocations and computations, but never sacrifice readability for insignificant optimizations.

Review and simplify. Always review your implementation and remove complexity before considering it finished.

The best code is code that needs no explanation.

## Building


```bash
cargo nice-plug bundle sozvezdie --release
```
