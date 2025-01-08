// Copied from https://github.com/maciejhirsz/logos/blob/master/tests/tests/ignore_case.rs
// and adapted for Herring.
//
// Copyright (c) 2018 Maciej Hirsz <maciej.hirsz@gmail.com>
//
// The MIT License (MIT)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

mod ignore_case {
    use herring::{assert_lex, Herring};

    #[derive(Herring, Debug, PartialEq, Eq)]
    #[herring(skip " +")]
    enum Words {
        #[token("élÉphAnt", ignore(case))]
        Elephant,
        #[token("ÉlèvE", ignore(case))]
        Eleve,
        #[token("à", ignore(case))]
        A,

        #[token("[abc]+", ignore(case))]
        Abc,
    }

    #[test]
    fn tokens() {
        assert_lex(
            "ÉLÉPHANT Éléphant ÉLèVE à À a",
            &[
                (Ok(Words::Elephant), "ÉLÉPHANT", 0..10),
                (Ok(Words::Elephant), "Éléphant", 11..21),
                (Ok(Words::Eleve), "ÉLèVE", 22..29),
                (Ok(Words::A), "à", 30..32),
                (Ok(Words::A), "À", 33..35),
                (Err(()), "a", 36..37),
            ],
        )
    }

    #[test]
    fn tokens_regex_escaped() {
        assert_lex(
            "[abc]+ abccBA",
            &[
                (Ok(Words::Abc), "[abc]+", 0..6),
                (Err(()), "a", 7..8),
                (Err(()), "b", 8..9),
                (Err(()), "c", 9..10),
                (Err(()), "c", 10..11),
                (Err(()), "B", 11..12),
                (Err(()), "A", 12..13),
            ],
        )
    }

    #[derive(Herring, PartialEq, Eq, Debug)]
    #[herring(skip " +")]
    enum Sink {
        #[regex("[abcéà]+", ignore(case))]
        Letters,
        #[regex("[0-9]+", ignore(case))]
        Numbers,
        #[regex("ééààé", ignore(case))]
        Sequence,
    }

    #[test]
    fn regex() {
        assert_lex(
            "aabbccééààéé 00123 ééààé ABCÉÀÀ ÉÉàÀÉ",
            &[
                (Ok(Sink::Letters), "aabbccééààéé", 0..18),
                (Ok(Sink::Numbers), "00123", 19..24),
                (Ok(Sink::Sequence), "ééààé", 25..35),
                (Ok(Sink::Letters), "ABCÉÀÀ", 36..45),
                (Ok(Sink::Sequence), "ÉÉàÀÉ", 46..56),
            ],
        )
    }
}
