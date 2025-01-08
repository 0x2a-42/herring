#![forbid(unsafe_code)]

mod debug;
mod dfa;
mod nfa;

use regex_syntax::hir::{ClassBytes, ClassBytesRange};
use regex_syntax::utf8::Utf8Range;
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

#[derive(Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct StateRef(usize);

#[derive(Clone, Default, Debug)]
pub struct State {
    transitions: Vec<Transition>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Pattern(ClassBytes);

#[derive(Clone, Debug)]
pub struct Transition {
    when: Pattern,
    to: StateRef,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Output {
    priority: usize,
    value: (String, usize),
}

#[derive(Clone, Debug)]
pub struct Automaton<const IS_DETERMINISTIC: bool> {
    start: StateRef,
    accepts: HashMap<StateRef, Option<Output>>,
    states: Vec<State>,
}
pub type Nfa = Automaton<false>;
pub type Dfa = Automaton<true>;

pub struct Token {
    nfa: Nfa,
    priority: usize,
    value: (String, usize),
}

impl<T: std::fmt::Display> From<T> for Error {
    fn from(value: T) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl StateRef {
    pub fn new(num: usize) -> Self {
        Self(num)
    }
    pub fn value(self) -> usize {
        self.0
    }
}

impl Pattern {
    fn from_byte(b: u8) -> Self {
        Self(ClassBytes::new([ClassBytesRange::new(b, b)]))
    }
    fn from_class(class: ClassBytes) -> Self {
        Self(class)
    }
    fn from_range(range: Utf8Range) -> Self {
        Self(ClassBytes::new([ClassBytesRange::new(
            range.start,
            range.end,
        )]))
    }
    fn empty() -> Self {
        Self(ClassBytes::empty())
    }
    pub fn contains(&self, b: u8) -> bool {
        self.0
            .ranges()
            .iter()
            .any(|r| r.start() <= b && b <= r.end())
    }
    fn is_empty(&self) -> bool {
        self.0.ranges().is_empty()
    }
    fn union(&mut self, other: &Pattern) {
        self.0.union(&other.0);
    }
    pub fn ranges(&self) -> &[ClassBytesRange] {
        self.0.ranges()
    }
}
impl PartialOrd for Pattern {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Pattern {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.ranges().cmp(other.0.ranges())
    }
}

impl Output {
    fn new(priority: usize, value: (String, usize)) -> Self {
        Self { priority, value }
    }
    pub fn value(&self) -> &(String, usize) {
        &self.value
    }
}

impl State {
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }
    fn new() -> Self {
        Self::default()
    }
}

impl Transition {
    pub fn when(&self) -> &Pattern {
        &self.when
    }
    pub fn to(&self) -> StateRef {
        self.to
    }
    fn new(when: Pattern, to: StateRef) -> Self {
        Self { when, to }
    }
}

impl<const IS_DETERMINISTIC: bool> Automaton<IS_DETERMINISTIC> {
    pub fn start(&self) -> StateRef {
        self.start
    }
    pub fn accepts(&self) -> &HashMap<StateRef, Option<Output>> {
        &self.accepts
    }
    pub fn states(&self) -> &[State] {
        &self.states
    }
    fn new() -> Self {
        Self {
            start: StateRef(0),
            accepts: HashMap::new(),
            states: vec![State::new()],
        }
    }
    fn add(&mut self) -> StateRef {
        let id = StateRef(self.states.len());
        self.states.push(State::new());
        id
    }
    fn add_accept(&mut self) -> StateRef {
        let id = self.add();
        self.set_accept(id);
        id
    }
    fn add_transition(&mut self, from: StateRef, when: Pattern, to: StateRef) {
        if IS_DETERMINISTIC && when.is_empty() {
            panic!("cannot add epsilon transition to DFA");
        }
        if let Some(t) = self.states[from.0]
            .transitions
            .iter_mut()
            .find(|t| t.to == to && ((!t.when.is_empty() && !when.is_empty()) || t.when == when))
        {
            t.when.union(&when);
        } else {
            self.states[from.0]
                .transitions
                .push(Transition::new(when, to))
        }
    }
    fn set_accept(&mut self, node: StateRef) {
        self.accepts.insert(node, None);
    }
    fn set_accept_output(&mut self, node: StateRef, output: Option<Output>) -> Result<(), Error> {
        if let Some(ref output) = output {
            if let Some(Some(current_output)) = self.accepts.get(&node) {
                match current_output.priority.cmp(&output.priority) {
                    Ordering::Less => {}
                    Ordering::Equal => {
                        return Err(format!(
                            "tokens `{}` and `{}` both have priority {} and may match the same word",
                            current_output.value.0, output.value.0, current_output.priority,
                        )
                        .into());
                    }
                    Ordering::Greater => return Ok(()),
                }
            }
        }
        self.accepts.insert(node, output);
        Ok(())
    }
    fn append(&mut self, other: Nfa) -> StateRef {
        let offset = self.states.len();
        for mut state in other.states {
            for t in state.transitions.iter_mut() {
                t.to.0 += offset;
            }
            self.states.push(state);
        }
        for (node, tok) in other.accepts {
            self.accepts.insert(StateRef(node.0 + offset), tok);
        }
        StateRef(other.start.0 + offset)
    }
}

impl Token {
    pub fn new(nfa: Nfa, priority: usize, value: (String, usize)) -> Self {
        Self {
            nfa,
            priority,
            value,
        }
    }
}
