# twotetrated

> A self-made compiler for a toy language for a self-made processor for a toy VM.

This project is a compiler (i.e. parser, compiler, and linker) for a new language "twotetrated". The idea is to provide a much more accessible way to develop programs for [TinyVM](https://github.com/BenWiederhake/tinyvm/).

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [Performance](#performance)
- [TODOs](#todos)
- [NOTDOs](#notdos)
- [Contribute](#contribute)

## Install

This project is in its infancy, so it's not available on crates.io yet.

However, you can use it anyway: Add at an appropriate position to your `Cargo.toml`:

```TOML
[dependencies]
subint = { git = "https://github.com/BenWiederhake/subint.git" }
```

That should be it. Since there are no versions yet, and no guarantee of stability
or compatibility whatsoever, you probably want to pin a specific commit.

## Usage

TODO

## Background

### Why though?

[TinyVM](https://github.com/BenWiederhake/tinyvm/) is 16-bit-everything.

Taking the second power of x is called "x squared", taking the third power of x is called "x cubed",
taking the fourth power is called "x tetrated". Therefore, two tetrated is 16.

TinyVM has a small and well-defined [Instruction Set](https://github.com/BenWiederhake/tinyvm/blob/master/instruction-set-architecture.md#instruction-set-architecture).

### Language design and specification

Current specification can be found in [`doc/language.md`](doc/language.md).

Decision and design documentation can be found in [`doc/adr/*`](doc/adr/).

TODO: Specify SSA-format, language, features, and even name!

### More

TODO

## TODOs

* ALL THE THINGS!!!

## NOTDOs

Here are some things this project will definitely not support:
* Anything with AI.
* Connections to the "outside world". The point of TinyVM is to have a safe and "obviously secure" sandbox, and programs in it can be easily and objectively measured. Providing any kind of bindings would defeat the purpose.
* Anything like multi-threading

These are highly unlikely, but you're welcome to develop it and make a PR (but please ask first so we can coordinate):
* Adaptions to the "SSA"-format so that integration with other compilers/languages is possible/easy
* LSP
* Integration with other languages
* Profiler/debugger/etc

## Contribute

Feel free to dive in! [Open an issue](https://github.com/BenWiederhake/twotetrated/issues/new) or submit PRs.
