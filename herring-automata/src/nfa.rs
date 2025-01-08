use crate::{Dfa, Error, Nfa, Output, Pattern, StateRef, Token};
use regex_syntax::hir::{Class, ClassUnicode, Hir, HirKind};
use regex_syntax::utf8::{Utf8Range, Utf8Sequence, Utf8Sequences};
use std::collections::{BTreeSet, HashMap};

impl Nfa {
    pub fn new_tokenizer(token_regexes: Vec<Token>) -> Nfa {
        let mut automaton = Nfa::new();
        for mut token_regex in token_regexes.into_iter() {
            for accept in token_regex.nfa.accepts.iter_mut() {
                *accept.1 = Some(Output::new(token_regex.priority, token_regex.value.clone()));
            }
            let s = automaton.append(token_regex.nfa);
            automaton.add_epsilon_transition(automaton.start, s);
        }
        automaton
    }

    pub fn accepts_empty(&self) -> bool {
        let mut start_set = BTreeSet::from_iter([self.start]);
        self.epsilon_closure(&mut start_set);
        for state in start_set {
            if self.accepts.contains_key(&state) {
                return true;
            }
        }
        false
    }

    /// Regex priority heuristic used by Logos
    fn hir_priority(hir: &Hir) -> usize {
        match hir.kind() {
            HirKind::Empty | HirKind::Look(_) => 0,
            HirKind::Literal(literal) => match std::str::from_utf8(&literal.0) {
                Ok(s) => 2 * s.chars().count(),
                Err(_) => 2 * literal.0.len(),
            },
            HirKind::Class(_) => 2,
            HirKind::Capture(capture) => Self::hir_priority(&capture.sub),
            HirKind::Concat(concat) => concat.iter().map(Self::hir_priority).sum(),
            HirKind::Alternation(alternation) => alternation
                .iter()
                .map(Self::hir_priority)
                .min()
                .unwrap_or(0),
            HirKind::Repetition(repetition) => {
                repetition.min as usize * Self::hir_priority(&repetition.sub)
            }
        }
    }

    fn replace_subpatterns(
        mut value: String,
        subpatterns: &HashMap<String, String>,
    ) -> Result<String, Error> {
        let mut replaced = true;
        while replaced {
            replaced = false;
            for (name, pattern) in subpatterns {
                let pattern_ref = format!("(?&{name})");
                let pattern = format!("({pattern})");
                if value.contains(&pattern_ref) {
                    replaced = true;
                    value = value.replace(&pattern_ref, &pattern);
                }
            }
        }
        if let Some(start) = value.find("(?&") {
            if let Some(end) = value[start..].find(')') {
                return Err(format!(
                    "use of undefined subpattern `{}`",
                    &value[start + 3..start + end]
                )
                .into());
            }
            return Err("use of undefined subpattern".into());
        }
        Ok(value)
    }

    pub fn from_regex_with_subpatterns(
        regex: &str,
        subpatterns: &HashMap<String, String>,
        ignore_case: bool,
        binary: bool,
    ) -> Result<(Nfa, usize), Error> {
        let subpattern_replaced_regex = Self::replace_subpatterns(regex.to_string(), subpatterns)?;
        Self::from_regex(&subpattern_replaced_regex, ignore_case, binary)
    }

    pub fn from_regex(regex: &str, ignore_case: bool, binary: bool) -> Result<(Nfa, usize), Error> {
        let hir = regex_syntax::ParserBuilder::new()
            .utf8(!binary)
            .unicode(!binary)
            .case_insensitive(ignore_case)
            .build()
            .parse(regex)?;
        let priority = Self::hir_priority(&hir);
        Ok((Self::from_hir(hir)?, priority))
    }

    pub fn from_token(token: &str, ignore_case: bool) -> (Nfa, usize) {
        Self::from_regex(&regex_syntax::escape(token), ignore_case, false).unwrap()
    }

    pub fn from_bytes(bytes: &[u8], ignore_case: bool) -> Nfa {
        let mut automaton = Nfa::new();
        let mut last_node = automaton.start;
        for b in bytes {
            let next = automaton.add();
            let mut pattern = Pattern::from_byte(*b);
            if ignore_case {
                pattern.0.case_fold_simple();
            }
            automaton.add_transition(last_node, pattern, next);
            last_node = next;
        }
        automaton.set_accept(last_node);
        automaton
    }

    /// Extended Thompson's construction
    fn from_hir(hir: Hir) -> Result<Nfa, Error> {
        match hir.into_kind() {
            HirKind::Empty => {
                let mut automaton = Nfa::new();
                let start = automaton.start;
                let end = automaton.add_accept();
                automaton.add_epsilon_transition(start, end);
                Ok(automaton)
            }
            HirKind::Literal(literal) => Ok(Self::from_bytes(&literal.0, false)),
            HirKind::Class(class) => Ok(match class {
                Class::Unicode(class) => Self::from_unicode_class(class),
                Class::Bytes(class) => {
                    let mut automaton = Nfa::new();
                    let start = automaton.start;
                    let end = automaton.add_accept();
                    automaton.add_transition(start, Pattern::from_class(class), end);
                    automaton
                }
            }),
            HirKind::Look(_) => Err("Herring does not support look-around".into()),
            HirKind::Repetition(repetition) => {
                if !repetition.greedy {
                    return Err("Herring does not support non-greedy repetitions".into());
                }
                Ok(match (repetition.min, repetition.max) {
                    (0, Some(1)) => {
                        let mut automaton = Self::from_hir(*repetition.sub)?;
                        let accepts = automaton.accepts.keys().copied().collect::<Vec<_>>();
                        for node in accepts {
                            automaton.add_epsilon_transition(automaton.start, node);
                        }
                        automaton
                    }
                    (0, None) => {
                        let mut inner = Self::from_hir(*repetition.sub)?;
                        let inner_accepts = inner.accepts.keys().copied().collect::<Vec<_>>();
                        for node in inner_accepts {
                            inner.add_epsilon_transition(node, inner.start);
                        }

                        let mut automaton = Nfa::new();
                        automaton.set_accept(automaton.start);
                        automaton.concat(inner);
                        let accepts = std::mem::take(&mut automaton.accepts);
                        let end = automaton.add_accept();
                        for (node, _) in accepts {
                            automaton.add_epsilon_transition(node, end);
                        }
                        automaton.add_epsilon_transition(automaton.start, end);
                        automaton
                    }
                    (1, None) => {
                        let mut automaton = Self::from_hir(*repetition.sub)?;
                        let accepts = automaton.accepts.keys().copied().collect::<Vec<_>>();
                        for node in accepts {
                            automaton.add_epsilon_transition(node, automaton.start);
                        }
                        automaton
                    }
                    (n, m) => {
                        let inner = Self::from_hir(*repetition.sub)?;
                        let mut automaton = if n > 0 {
                            inner.clone()
                        } else {
                            let mut empty = Nfa::new();
                            empty.set_accept(empty.start);
                            empty
                        };
                        for _ in 1..n {
                            automaton.concat(inner.clone());
                        }
                        if let Some(m) = m {
                            if m > n {
                                let mut maybe = inner.clone();
                                let accepts = maybe.accepts.keys().copied().collect::<Vec<_>>();
                                for node in accepts {
                                    maybe.add_epsilon_transition(maybe.start, node);
                                }
                                for _ in n..m {
                                    automaton.concat(maybe.clone());
                                }
                            }
                        } else {
                            let mut repeat = inner.clone();
                            let accepts = repeat.accepts.keys().copied().collect::<Vec<_>>();
                            for node in accepts {
                                repeat.add_epsilon_transition(repeat.start, node);
                                repeat.add_epsilon_transition(node, repeat.start);
                            }
                            automaton.concat(repeat);
                        }
                        automaton
                    }
                })
            }
            HirKind::Capture(capture) => Self::from_hir(*capture.sub),
            HirKind::Concat(concat) => {
                let mut it = concat.into_iter();
                let mut automaton = Self::from_hir(it.next().unwrap())?;
                for a in it {
                    automaton.concat(Self::from_hir(a)?);
                }
                Ok(automaton)
            }
            HirKind::Alternation(alternation) => {
                let mut automaton = Nfa::new();
                for a in alternation.into_iter() {
                    let s = automaton.append(Self::from_hir(a)?);
                    automaton.add_epsilon_transition(automaton.start, s);
                }
                let old_accepts = std::mem::take(&mut automaton.accepts);
                let new_end = automaton.add();
                for (node, _) in old_accepts {
                    automaton.add_epsilon_transition(node, new_end);
                }
                automaton.set_accept(new_end);
                Ok(automaton)
            }
        }
    }

    fn concat(&mut self, other: Nfa) {
        let old_accepts = std::mem::take(&mut self.accepts);
        let s = self.append(other);
        for (node, _) in old_accepts {
            self.add_epsilon_transition(node, s);
        }
    }

    fn add_epsilon_transition(&mut self, from: StateRef, to: StateRef) {
        self.add_transition(from, Pattern::empty(), to);
    }

    fn from_unicode_class(class: ClassUnicode) -> Nfa {
        let mut automaton = Nfa::new();
        let start = automaton.start;
        let end = automaton.add();

        fn to_key(ranges: &[Utf8Range]) -> u64 {
            assert!(ranges.len() <= 4);
            let mut key = 0u64;
            for range in ranges {
                key <<= 8;
                key |= range.start as u64;
                key <<= 8;
                key |= range.end as u64;
            }
            key
        }
        let mut suffix = HashMap::<u64, StateRef>::new();
        for r in class.ranges() {
            for seq in Utf8Sequences::new(r.start(), r.end()) {
                match seq {
                    Utf8Sequence::One(range) => {
                        automaton.add_transition(start, Pattern::from_range(range), end)
                    }
                    Utf8Sequence::Two([range1, range2]) => {
                        let state = *suffix
                            .entry(to_key(&[range2]))
                            .or_insert_with(|| automaton.add());
                        automaton.add_transition(start, Pattern::from_range(range1), state);
                        automaton.add_transition(state, Pattern::from_range(range2), end);
                    }
                    Utf8Sequence::Three([range1, range2, range3]) => {
                        let state1 = *suffix
                            .entry(to_key(&[range2, range3]))
                            .or_insert_with(|| automaton.add());
                        let state2 = *suffix
                            .entry(to_key(&[range3]))
                            .or_insert_with(|| automaton.add());
                        automaton.add_transition(start, Pattern::from_range(range1), state1);
                        automaton.add_transition(state1, Pattern::from_range(range2), state2);
                        automaton.add_transition(state2, Pattern::from_range(range3), end);
                    }
                    Utf8Sequence::Four([range1, range2, range3, range4]) => {
                        let state1 = *suffix
                            .entry(to_key(&[range2, range3, range4]))
                            .or_insert_with(|| automaton.add());
                        let state2 = *suffix
                            .entry(to_key(&[range3, range4]))
                            .or_insert_with(|| automaton.add());
                        let state3 = *suffix
                            .entry(to_key(&[range4]))
                            .or_insert_with(|| automaton.add());
                        automaton.add_transition(start, Pattern::from_range(range1), state1);
                        automaton.add_transition(state1, Pattern::from_range(range2), state2);
                        automaton.add_transition(state2, Pattern::from_range(range3), state3);
                        automaton.add_transition(state3, Pattern::from_range(range4), end);
                    }
                }
            }
        }
        automaton.set_accept(end);
        automaton
    }

    fn epsilon_closure(&self, state_set: &mut BTreeSet<StateRef>) {
        let mut size = state_set.len();
        loop {
            let mut new_states = vec![];
            for state in state_set.iter() {
                for transition in self.states[state.0].transitions.iter() {
                    if transition.when.is_empty() {
                        new_states.push(transition.to);
                    }
                }
            }
            state_set.extend(new_states);
            if size == state_set.len() {
                break;
            }
            size = state_set.len();
        }
    }

    fn move_set(&self, state_set: &BTreeSet<StateRef>, b: u8) -> BTreeSet<StateRef> {
        let mut set = BTreeSet::new();
        for state in state_set.iter() {
            for t in self.states[state.0].transitions.iter() {
                if t.when.contains(b) {
                    set.insert(t.to);
                }
            }
        }
        self.epsilon_closure(&mut set);
        set
    }

    /// Subset construction
    pub fn into_dfa(self) -> Result<Dfa, Error> {
        let mut automaton = Dfa::new();

        let mut start_set = BTreeSet::from_iter([self.start]);
        self.epsilon_closure(&mut start_set);

        let mut dstates = HashMap::new();
        dstates.insert(start_set.clone(), self.start);
        let mut todo = vec![start_set];
        while let Some(state_set) = todo.pop() {
            let dfa_state = *dstates.get(&state_set).unwrap();
            for (accept, tok) in self.accepts.iter() {
                if state_set.contains(accept) {
                    automaton.set_accept_output(dfa_state, tok.clone())?;
                }
            }
            for b in u8::MIN..=u8::MAX {
                let move_set = self.move_set(&state_set, b);
                if move_set.is_empty() {
                    continue;
                }
                if let Some(next_dfa_state) = dstates.get(&move_set) {
                    automaton.add_transition(dfa_state, Pattern::from_byte(b), *next_dfa_state);
                    continue;
                }
                let next_dfa_state = automaton.add();
                dstates.insert(move_set.clone(), next_dfa_state);
                todo.push(move_set);
                automaton.add_transition(dfa_state, Pattern::from_byte(b), next_dfa_state);
            }
        }
        Ok(automaton)
    }
}
