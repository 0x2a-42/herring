use crate::{Dfa, Output, StateRef};
use std::collections::{BTreeSet, HashMap};

impl Dfa {
    /// Hopcroft's algorithm
    pub fn into_minimized(self) -> Self {
        let mut accept_partitions = HashMap::<Output, Vec<StateRef>>::new();
        for (state, output) in self.accepts.iter() {
            if let Some(output) = output {
                accept_partitions
                    .entry(output.clone())
                    .and_modify(|e| e.push(*state))
                    .or_insert(vec![*state]);
            }
        }
        let mut p = BTreeSet::new();
        for (_, states) in accept_partitions {
            p.insert(BTreeSet::from_iter(states));
        }
        let mut non_accept = BTreeSet::new();
        for s in 0..self.states.len() {
            if !self.accepts.contains_key(&StateRef(s)) {
                non_accept.insert(StateRef(s));
            }
        }
        if !non_accept.is_empty() {
            p.insert(non_accept);
        }
        let mut w = p.clone();

        while !w.is_empty() {
            let a = w.pop_last().unwrap();
            for b in u8::MIN..=u8::MAX {
                let mut x = BTreeSet::new();
                for (num, s) in self.states.iter().enumerate() {
                    for t in s.transitions.iter() {
                        let from = StateRef(num);
                        if a.contains(&t.to) && t.when.contains(b) {
                            x.insert(from);
                        }
                    }
                }
                if x.is_empty() {
                    continue;
                }
                let mut replacements = Vec::with_capacity(p.len());
                for y in p.iter() {
                    let cut = x.intersection(y).copied().collect::<BTreeSet<_>>();
                    let diff = y.difference(&x).copied().collect::<BTreeSet<_>>();
                    if !cut.is_empty() && !diff.is_empty() {
                        if w.contains(y) {
                            w.remove(y);
                            w.insert(cut.clone());
                            w.insert(diff.clone());
                        } else if cut.len() < diff.len() {
                            w.insert(cut.clone());
                        } else {
                            w.insert(diff.clone());
                        }
                        replacements.push((y.clone(), (cut, diff)));
                    }
                }
                for (y, (cut, diff)) in replacements.into_iter() {
                    p.remove(&y);
                    p.insert(cut);
                    p.insert(diff);
                }
            }
        }

        let mut automaton = Dfa::new();
        let mut new_states = HashMap::new();
        for partition in p {
            let state = if partition.contains(&self.start) {
                automaton.start
            } else {
                automaton.add()
            };
            new_states.insert(partition, state);
        }
        for (partition, state) in new_states.iter() {
            for s in partition {
                if let Some(tok) = self.accepts.get(s) {
                    let _ = automaton.set_accept_output(*state, tok.clone());
                }
                for t in self.states[s.0].transitions.iter() {
                    let to = new_states
                        .iter()
                        .find_map(|(p, s)| p.contains(&t.to).then_some(*s))
                        .unwrap();
                    automaton.add_transition(*state, t.when.clone(), to);
                }
            }
        }
        automaton
    }
}
