# Language

This document describes the language "twotetrated", as accepted and processed by the twotetrated compiler.
Note that this is distinct from linking, which is described in FIXME.

## Grammar

Auto-generated from [`src/parser.rs`](../src/parser.rs). See [`language_grammar.txt`] for an EBNF-like description. Keep in mind that this is NOT a context-free-grammar, but rather a PEG grammar.

## Corner-stones

- Imperative: More accessible to everyone
- Statically-typed: Simpler translation to assembly, the only thing I dislike about Python are its dynamic types, and dynamic types would require at least *some* overhead, which would be too much on TinyVM.
- Strongly typed: Just look at how JavaScript is faring with its "[] + {} is a legal expression" approach.
- No fancy features for now: Just a minimal product for now. Maybe later?
    * Includes inheritance, vtables, object oriented programming, references, templates?, macros?, etc.
- No "Undefined Behavior". Either it's a compilation error, or it's clear and obvious what the program will do.
- Spell it out. No `&'a mut ref fn impl wklrghrg" garbage, but actually readable source code for humans.
    * This also affects pointers, especially since we have distinct data and instruction memory segments.
- Sensible order: Rust already gets it quite well, but I think it's still not quite there yet.
    Go makes the controversial move of putting the `[]` brackets *before* the typename, and I think that's the correct move. Something like this.
- Something about declarations vs. definitions, and how to share it between compilation units.
    * Don't like the Rust/Java approach where "compiler does everything", and you just have to accept it.
    * Don't like the C/C++ approach where header files contain mixed declarations/definitions.
    * Also, how to deal with cyclic dependencies? LinkedLists must be allowed to exist!
    * One option: Declarations go here (`*.tth`), definitions (and private declarations) go there (`*.tt`).
        However, struct *definitions* are also allowed to exist in the header. Cycles are forbidden,
        but shared subtrees are automatically pruned away (like the #ifndef/#define pattern in C.)
