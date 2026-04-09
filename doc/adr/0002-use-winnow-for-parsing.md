# 2. Use winnow for parsing

Date: 2026-04-09

## Status

Accepted

## Context

The first step for any compiler is to somehow get the raw, human-readable source code, into memory, somehow.
This is a tiny project with little manpower, so ease-of-use and low implementation cost is a must.
This is a toy project for tiny programs running on a tiny architecture, so speed is entirely optional.
I hope that other people will use this project, so it must be possible (and ideally convenient)
to provide "nice" compiler errors to the user. That includes parsing errors, token span tracking, etc.
I'm a hard-ass on correctness, so I strongly prefer being able to review the effective grammar,
and making sure that issues like "the dangling else problem" are recognized in advance,
and can be approached with a conscious decision.

## Disregarded alternatives

- self-made: No. Too much effort, and I would probably choose horrible designs (not usable),
  I would need to re-invent a lot of parsing theory.
- LALR-style parsers like bison/yacc/antlr/Lark: Ugly in my experience, resolving grammar issues is a pain,
  need to rephrase the grammar a whole lot just to conform to LALR-rules, unclear which libraries provide good token tracking.
- Therefore, PEG parsers seem like a better choice (but have the disadvantage of being harder to "prove correct").
- pest: Looks good at first glance, but it essentially gives only a stream of tokens ("Pair"),
  so the grammar has to be implemented twice. Too much room for error, also it doesn't seem actively maintained anymore.
- nom: Looks even better at first glance, but there seems to be something dark lurking in the waters:
  Maintenance suddenly stopped, the entire project got forked.

## Decision

Use winnow, the fork of nom, which has been actively maintained and furthered for several years.
Winnow has some nice projects built on top of it, and it seems to have built-in support for token span
tracking and error reporting. (nom would require reinventing the wheel by finding and composing the correct crates.)

Use it for: parsing, parse error reporting, token span tracking.

Use magic comments in `parser.rs` to support "extracting" the factual grammar. This means that we still have the grammar
explicitly available, for potential later analysis.

## Consequences

- Easier: Implementation, parse error reporting, token span tracking.
- More difficult: Grammar correctness analysis, checking for "ambiguities" (because PEG grammars technically are ambiguity-free
  in the worst possible interpretation)
- Risks: Winnow becomes abandoned and unusable in some form?
- Easier: Finding other people who might be interested to jump on?
