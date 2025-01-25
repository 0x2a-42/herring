# Herring
[![Crates.io](https://img.shields.io/crates/v/herring)](https://crates.io/crates/herring)
[![MIT/Apache 2.0](https://img.shields.io/crates/l/herring)](./LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/d/herring)](https://crates.io/crates/herring)
<!--[![Rust](https://img.shields.io/github/actions/workflow/status/0x2a-42/herring/rust.yml)](https://github.com/0x2a-42/herring/actions)-->

[Herring](https://en.wikipedia.org/wiki/Herring) (**H**ighly **E**fficient **R**ust **R**egex-based lexer **I**mplementatio**N** **G**enerator) is a lexer generator for Rust implementing a subset of the [Logos](https://github.com/maciejhirsz/logos) API.

The key differences compared to Logos are the following.

- The `Herring` trait must be derived on a [unit-only enum](https://doc.rust-lang.org/reference/items/enumerations.html#r-items.enum.unit-only).
- Only `Result<TokenType, ErrorType>` is allowed for a `regex` or `token` callback return type.
- Lexer modes (`morph` method) are not supported (use callbacks and `extras` instead).
- There is no `ignore(ascii_case)`, only `ignore(case)`.

These changes are mostly due to the use case as a lexer for the [Lelwel](https://github.com/0x2a-42/lelwel) parser generator, where unit-only enums are required.
There are also additional features that are not available in Logos.

- There is an `ignore` callback that can be used to skip input before it is passed to the automaton (e.g. for lexing [escaped newlines](./tests/ignore.rs) in C).
- There is an `initial` callback that can be used for generating tokens without consuming input (e.g. for [indent and dedent tokens](./tests/initial.rs) in Python).
- A callback with unit return type can be specified for `skip` regexes.

> [!WARNING]
> At the moment you should almost certainly use Logos instead of Herring, as it is more mature and provides better performance.
> There are however also valid reasons for using Herring.
> There are currently some bugs in Logos related to [backtracking](https://github.com/maciejhirsz/logos/issues?q=is%3Aissue+is%3Aopen+backtracking) and [stack overflows](https://github.com/maciejhirsz/logos/issues?q=is%3Aissue+is%3Aopen+stack+overflow) that [are not present](./tests/logos_bugs.rs) in Herring.
> You may also use Herring if you require the `ignore` or `initial` callbacks.

## Performance
### Runtime
Herring implements some of the optimizations also present in Logos.
These are jump tables for states with many outgoing transitions and stacked lookup tables for transitions with complicated byte ranges.
It does currently not implement some other optimizations such as loop unrolling or transitioning on strings.

#### DFA Jump Threading
Unlike Logos, which encodes its state machine with mutually tail recursive functions, Herring uses a `match` inside of a `loop`.
This has the advantage, that the state machine will not overflow the stack on long inputs, if it is compiled without optimizations.
On the other hand a disadvantage is, that Rust currently does not optimize away the extra jumps (see https://github.com/rust-lang/rust/issues/80630 and https://github.com/rust-lang/rfcs/pull/3720).
By passing [`-enable-dfa-jump-thread`](https://github.com/llvm/llvm-project/commit/02077da7e7a8ff76c0576bb33adb462c337013f5) to LLVM it is however possible to enable this optimization on the LLVM level.
This can be achieved by adding a [`.cargo/config.toml`](./.cargo/config.toml) file to your crate.

> [!CAUTION]
> Be aware, that adding LLVM passes to the rustc optimization pipeline is probably not well tested and may increase the probability for running into compiler bugs.

#### Benchmark
The results of the [Logos benchmark](./benches/logos_benchmark.rs) on an Intel Core i7-8550U CPU are shown in the following table.
Currently in most cases Logos is faster than Herring with DFA jump threading.

| Benchmark | Without DFA Jump Threading | With DFA Jump Threading | Logos |
| --- | --- | --- | --- |
| `iterate/identifiers` | 1.9877 µs (373.75 MiB/s) | **878.93 ns (845.24 MiB/s)** | 951.53 ns (780.75 MiB/s) |
| `iterate/keywords_operators_and_punctators` | 6.1906 µs (328.28 MiB/s) | 2.8433 µs (714.76 MiB/s) | **2.6915 µs (755.08 MiB/s)** |
| `iterate/strings` | 2.0411 µs (406.97 MiB/s) | 1.0142 µs (818.99 MiB/s) | **717.31 ns (1.1309 GiB/s)** |

### Compile Time
Using complex unicode ranges or finite repetitions may result in large NFAs, for which the transformation to a minimized DFA may take a noticeable amount of time.
To improve build times you can add the following to your `Cargo.toml` file, so the procedural macro code is optimized.
```toml
[profile.dev.build-override]
opt-level = 3

[profile.release.build-override]
opt-level = 3
```

## Example
The Herring API is almost identical to the Logos API, so it can be used as a drop in replacement, if the above mentioned restrictions apply.

The following example shows a lexer for JSON (string escape sequences are not validated, as this is better handled [after lexing](https://github.com/0x2a-42/lelwel/blob/3b1abbf4f717bdcff1e9f66f05086e7365410eee/examples/json/src/parser.rs#L97-L145), to avoid tripping up the parser with a missing string token).
```rust
use herring::{Herring, Lexer};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum LexerError {
    #[default]
    Invalid,
    UnterminatedString,
}
fn parse_string(lexer: &mut Lexer<'_, Token>) -> Result<Token, LexerError> {
    let mut it = lexer.remainder().chars();
    while let Some(c) = it.next() {
        match c {
            '"' => {
                lexer.bump(1);
                return Ok(Token::String);
            }
            '\\' => {
                lexer.bump(1);
                if let Some(c) = it.next() {
                    lexer.bump(c.len_utf8());
                }
            }
            c => lexer.bump(c.len_utf8()),
        }
    }
    Err(LexerError::UnterminatedString)
}
#[derive(Herring, Debug, PartialEq, Copy, Clone)]
#[herring(error = LexerError)]
pub enum Token {
    #[regex("[\u{0020}\u{000A}\u{000D}\u{0009}]+")]
    Whitespace,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("null")]
    Null,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBrak,
    #[token("]")]
    RBrak,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[regex("\"", parse_string)]
    String,
    #[regex(r"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    Number,
}
fn main() {
    for tok in Token::lexer(r#"{"test": [1,2,3]}"#) {
        println!("{tok:?}");
    }
}
```

## Debugging
You can inspect the generated code by running [`cargo expand`](https://github.com/dtolnay/cargo-expand).

You can generate [Graphviz](https://graphviz.org/) or [Mermaid](https://mermaid.js.org/) graphs by setting the environment variable `HERRING_DEBUG` to either `graphviz` or `mermaid` during the execution of the procedural macro.
This generates a file for the NFA (`nfa.dot` or `nfa.mmd`), the subset construction DFA (`dfa.dot` or `dfa.mmd`), and the minimized DFA (`min.dot` or `min.mmd`) in the specified format.
Graphviz is useful when debugging locally with tools like [`xdot`](https://github.com/jrfonseca/xdot.py).
Mermaid is useful for directly embedding graphs in Github issues.

Setting `HERRING_DEBUG` to `log` makes the lexer write all visited states to stderr.

> [!NOTE]
> When changing the `HERRING_DEBUG` environment variable you have to make sure that the build of the lexer is not skipped by `cargo build`.
> If you made no changes since the last build you can safe the containing file without any changes to trigger a rebuild.

### JSON Example
#### NFA
```mermaid
flowchart LR
style start fill:#FFFFFF00, stroke:#FFFFFF00
start-->0;
0@{shape: circ}
0 -- "ε" --> 1
0 -- "ε" --> 3
0 -- "ε" --> 8
0 -- "ε" --> 14
0 -- "ε" --> 19
0 -- "ε" --> 21
0 -- "ε" --> 23
0 -- "ε" --> 25
0 -- "ε" --> 27
0 -- "ε" --> 29
0 -- "ε" --> 31
0 -- "ε" --> 33
1@{shape: circ}
1 -- "{0x9-0xA, 0xD, 0x20}" --> 2
2@{shape: dbl-circ}
2 -- "ε" --> 1
3@{shape: circ}
3 -- "{'t'}" --> 4
4@{shape: circ}
4 -- "{'r'}" --> 5
5@{shape: circ}
5 -- "{'u'}" --> 6
6@{shape: circ}
6 -- "{'e'}" --> 7
7@{shape: dbl-circ}
8@{shape: circ}
8 -- "{'f'}" --> 9
9@{shape: circ}
9 -- "{'a'}" --> 10
10@{shape: circ}
10 -- "{'l'}" --> 11
11@{shape: circ}
11 -- "{'s'}" --> 12
12@{shape: circ}
12 -- "{'e'}" --> 13
13@{shape: dbl-circ}
14@{shape: circ}
14 -- "{'n'}" --> 15
15@{shape: circ}
15 -- "{'u'}" --> 16
16@{shape: circ}
16 -- "{'l'}" --> 17
17@{shape: circ}
17 -- "{'l'}" --> 18
18@{shape: dbl-circ}
19@{shape: circ}
19 -- "{'{'}" --> 20
20@{shape: dbl-circ}
21@{shape: circ}
21 -- "{'}'}" --> 22
22@{shape: dbl-circ}
23@{shape: circ}
23 -- "{'['}" --> 24
24@{shape: dbl-circ}
25@{shape: circ}
25 -- "{']'}" --> 26
26@{shape: dbl-circ}
27@{shape: circ}
27 -- "{','}" --> 28
28@{shape: dbl-circ}
29@{shape: circ}
29 -- "{':'}" --> 30
30@{shape: dbl-circ}
31@{shape: circ}
31 -- "{'#34;'}" --> 32
32@{shape: dbl-circ}
33@{shape: circ}
33 -- "{'-'}" --> 34
33 -- "ε" --> 34
34@{shape: circ}
34 -- "ε" --> 35
35@{shape: circ}
35 -- "ε" --> 36
35 -- "ε" --> 38
36@{shape: circ}
36 -- "{'0'}" --> 37
37@{shape: circ}
37 -- "ε" --> 44
38@{shape: circ}
38 -- "{'1'-'9'}" --> 39
39@{shape: circ}
39 -- "ε" --> 40
40@{shape: circ}
40 -- "ε" --> 41
40 -- "ε" --> 43
41@{shape: circ}
41 -- "{'0'-'9'}" --> 42
42@{shape: circ}
42 -- "ε" --> 41
42 -- "ε" --> 43
43@{shape: circ}
43 -- "ε" --> 44
44@{shape: circ}
44 -- "ε" --> 45
45@{shape: circ}
45 -- "{'.'}" --> 46
45 -- "ε" --> 48
46@{shape: circ}
46 -- "ε" --> 47
47@{shape: circ}
47 -- "{'0'-'9'}" --> 48
48@{shape: circ}
48 -- "ε" --> 47
48 -- "ε" --> 49
49@{shape: circ}
49 -- "{'E', 'e'}" --> 50
49 -- "ε" --> 54
50@{shape: circ}
50 -- "ε" --> 51
51@{shape: circ}
51 -- "{'+', '-'}" --> 52
51 -- "ε" --> 52
52@{shape: circ}
52 -- "ε" --> 53
53@{shape: circ}
53 -- "{'0'-'9'}" --> 54
54@{shape: dbl-circ}
54 -- "ε" --> 53
String_0[String]@{shape: rect}
32 .-> String_0
Colon_0[Colon]@{shape: rect}
30 .-> Colon_0
Number_0[Number]@{shape: rect}
54 .-> Number_0
LBrace_0[LBrace]@{shape: rect}
20 .-> LBrace_0
LBrak_0[LBrak]@{shape: rect}
24 .-> LBrak_0
RBrak_0[RBrak]@{shape: rect}
26 .-> RBrak_0
Null_0[Null]@{shape: rect}
18 .-> Null_0
False_0[False]@{shape: rect}
13 .-> False_0
True_0[True]@{shape: rect}
7 .-> True_0
Whitespace_0[Whitespace]@{shape: rect}
2 .-> Whitespace_0
RBrace_0[RBrace]@{shape: rect}
22 .-> RBrace_0
Comma_0[Comma]@{shape: rect}
28 .-> Comma_0
```

#### Subset Construction DFA
```mermaid
flowchart LR
style start fill:#FFFFFF00, stroke:#FFFFFF00
start-->0;
0@{shape: circ}
0 -- "{0x9-0xA, 0xD, 0x20}" --> 1
0 -- "{'#34;'}" --> 2
0 -- "{','}" --> 3
0 -- "{'-'}" --> 4
0 -- "{'0'}" --> 5
0 -- "{'1'-'9'}" --> 6
0 -- "{':'}" --> 7
0 -- "{'['}" --> 8
0 -- "{']'}" --> 9
0 -- "{'f'}" --> 10
0 -- "{'n'}" --> 11
0 -- "{'t'}" --> 12
0 -- "{'{'}" --> 13
0 -- "{'}'}" --> 14
1@{shape: dbl-circ}
1 -- "{0x9-0xA, 0xD, 0x20}" --> 1
2@{shape: dbl-circ}
3@{shape: dbl-circ}
4@{shape: circ}
4 -- "{'0'}" --> 5
4 -- "{'1'-'9'}" --> 6
5@{shape: dbl-circ}
5 -- "{'.'}" --> 25
5 -- "{'0'-'9'}" --> 30
5 -- "{'E', 'e'}" --> 27
6@{shape: dbl-circ}
6 -- "{'.'}" --> 25
6 -- "{'0'-'9'}" --> 26
6 -- "{'E', 'e'}" --> 27
7@{shape: dbl-circ}
8@{shape: dbl-circ}
9@{shape: dbl-circ}
10@{shape: circ}
10 -- "{'a'}" --> 21
11@{shape: circ}
11 -- "{'u'}" --> 18
12@{shape: circ}
12 -- "{'r'}" --> 15
13@{shape: dbl-circ}
14@{shape: dbl-circ}
15@{shape: circ}
15 -- "{'u'}" --> 16
16@{shape: circ}
16 -- "{'e'}" --> 17
17@{shape: dbl-circ}
18@{shape: circ}
18 -- "{'l'}" --> 19
19@{shape: circ}
19 -- "{'l'}" --> 20
20@{shape: dbl-circ}
21@{shape: circ}
21 -- "{'l'}" --> 22
22@{shape: circ}
22 -- "{'s'}" --> 23
23@{shape: circ}
23 -- "{'e'}" --> 24
24@{shape: dbl-circ}
25@{shape: circ}
25 -- "{'0'-'9'}" --> 30
26@{shape: dbl-circ}
26 -- "{'.'}" --> 25
26 -- "{'0'-'9'}" --> 26
26 -- "{'E', 'e'}" --> 27
27@{shape: circ}
27 -- "{'+', '-'}" --> 28
27 -- "{'0'-'9'}" --> 29
28@{shape: circ}
28 -- "{'0'-'9'}" --> 29
29@{shape: dbl-circ}
29 -- "{'0'-'9'}" --> 29
30@{shape: dbl-circ}
30 -- "{'0'-'9'}" --> 30
30 -- "{'E', 'e'}" --> 27
LBrak_0[LBrak]@{shape: rect}
8 .-> LBrak_0
String_0[String]@{shape: rect}
2 .-> String_0
Number_0[Number]@{shape: rect}
30 .-> Number_0
Colon_0[Colon]@{shape: rect}
7 .-> Colon_0
Number_0[Number]@{shape: rect}
6 .-> Number_0
True_0[True]@{shape: rect}
17 .-> True_0
Null_0[Null]@{shape: rect}
20 .-> Null_0
False_0[False]@{shape: rect}
24 .-> False_0
Number_0[Number]@{shape: rect}
5 .-> Number_0
Number_0[Number]@{shape: rect}
29 .-> Number_0
Whitespace_0[Whitespace]@{shape: rect}
1 .-> Whitespace_0
RBrak_0[RBrak]@{shape: rect}
9 .-> RBrak_0
Number_0[Number]@{shape: rect}
26 .-> Number_0
RBrace_0[RBrace]@{shape: rect}
14 .-> RBrace_0
Comma_0[Comma]@{shape: rect}
3 .-> Comma_0
LBrace_0[LBrace]@{shape: rect}
13 .-> LBrace_0
```

#### Minimized DFA
```mermaid
flowchart LR
style start fill:#FFFFFF00, stroke:#FFFFFF00
start-->0;
0@{shape: circ}
0 -- "{0x9-0xA, 0xD, 0x20}" --> 1
0 -- "{'#34;'}" --> 2
0 -- "{','}" --> 3
0 -- "{'-'}" --> 4
0 -- "{'0'}" --> 5
0 -- "{'1'-'9'}" --> 6
0 -- "{':'}" --> 7
0 -- "{'['}" --> 8
0 -- "{']'}" --> 9
0 -- "{'f'}" --> 10
0 -- "{'n'}" --> 11
0 -- "{'t'}" --> 12
0 -- "{'{'}" --> 13
0 -- "{'}'}" --> 14
1@{shape: dbl-circ}
1 -- "{0x9-0xA, 0xD, 0x20}" --> 1
2@{shape: dbl-circ}
3@{shape: dbl-circ}
4@{shape: circ}
4 -- "{'0'}" --> 5
4 -- "{'1'-'9'}" --> 6
5@{shape: dbl-circ}
5 -- "{'.'}" --> 25
5 -- "{'0'-'9'}" --> 29
5 -- "{'E', 'e'}" --> 26
6@{shape: dbl-circ}
6 -- "{'.'}" --> 25
6 -- "{'0'-'9'}" --> 6
6 -- "{'E', 'e'}" --> 26
7@{shape: dbl-circ}
8@{shape: dbl-circ}
9@{shape: dbl-circ}
10@{shape: circ}
10 -- "{'a'}" --> 21
11@{shape: circ}
11 -- "{'u'}" --> 18
12@{shape: circ}
12 -- "{'r'}" --> 15
13@{shape: dbl-circ}
14@{shape: dbl-circ}
15@{shape: circ}
15 -- "{'u'}" --> 16
16@{shape: circ}
16 -- "{'e'}" --> 17
17@{shape: dbl-circ}
18@{shape: circ}
18 -- "{'l'}" --> 19
19@{shape: circ}
19 -- "{'l'}" --> 20
20@{shape: dbl-circ}
21@{shape: circ}
21 -- "{'l'}" --> 22
22@{shape: circ}
22 -- "{'s'}" --> 23
23@{shape: circ}
23 -- "{'e'}" --> 24
24@{shape: dbl-circ}
25@{shape: circ}
25 -- "{'0'-'9'}" --> 29
26@{shape: circ}
26 -- "{'+', '-'}" --> 27
26 -- "{'0'-'9'}" --> 28
27@{shape: circ}
27 -- "{'0'-'9'}" --> 28
28@{shape: dbl-circ}
28 -- "{'0'-'9'}" --> 28
29@{shape: dbl-circ}
29 -- "{'0'-'9'}" --> 29
29 -- "{'E', 'e'}" --> 26
False_0[False]@{shape: rect}
24 .-> False_0
Whitespace_0[Whitespace]@{shape: rect}
1 .-> Whitespace_0
Number_0[Number]@{shape: rect}
6 .-> Number_0
Number_0[Number]@{shape: rect}
29 .-> Number_0
LBrace_0[LBrace]@{shape: rect}
13 .-> LBrace_0
Number_0[Number]@{shape: rect}
5 .-> Number_0
RBrace_0[RBrace]@{shape: rect}
14 .-> RBrace_0
String_0[String]@{shape: rect}
2 .-> String_0
Colon_0[Colon]@{shape: rect}
7 .-> Colon_0
RBrak_0[RBrak]@{shape: rect}
9 .-> RBrak_0
True_0[True]@{shape: rect}
17 .-> True_0
Null_0[Null]@{shape: rect}
20 .-> Null_0
LBrak_0[LBrak]@{shape: rect}
8 .-> LBrak_0
Number_0[Number]@{shape: rect}
28 .-> Number_0
Comma_0[Comma]@{shape: rect}
3 .-> Comma_0
```

## License
Herring is licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
