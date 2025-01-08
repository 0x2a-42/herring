use crate::{Automaton, Output, Pattern};

impl core::fmt::Debug for Pattern {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        fn write_byte(f: &mut core::fmt::Formatter, b: u8) -> core::fmt::Result {
            if b.is_ascii_graphic() {
                write!(f, "'{}'", b as char)
            } else {
                write!(f, "0x{:X}", b)
            }
        }
        if self.is_empty() {
            return write!(f, "ε");
        }
        write!(f, "{{")?;
        for (i, r) in self.0.ranges().iter().enumerate() {
            let start = r.start();
            let end = r.end();
            if start == end {
                write_byte(f, start)?;
            } else {
                write_byte(f, start)?;
                write!(f, "-")?;
                write_byte(f, end)?;
            }
            if i + 1 < self.0.ranges().len() {
                write!(f, ", ")?
            }
        }
        write!(f, "}}")
    }
}

impl core::fmt::Debug for Output {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        if self.value.1 > 0 {
            write!(f, "{}: {}", self.value.0, self.value.1)
        } else {
            write!(f, "{}", self.value.0)
        }
    }
}

impl<const IS_DETERMINISTIC: bool> Automaton<IS_DETERMINISTIC> {
    pub fn print_graphviz(&self, name: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(name)?;

        writeln!(f, "digraph {{\nrankdir=LR;")?;
        writeln!(f, "start [shape=none];\nstart -> {};", self.start.0)?;
        for (i, node) in self.states.iter().enumerate() {
            writeln!(
                f,
                "{i} [shape={}];",
                if self.accepts.iter().any(|(node, _)| i == node.0) {
                    "doublecircle"
                } else {
                    "circle"
                }
            )?;
            for t in node.transitions.iter() {
                let label = format!("{:?}", t.when)
                    .replace("\"", "\\\"")
                    .replace("\\", "\\\\");
                writeln!(f, "{i} -> {} [label=\"{label}\"];", t.to.0)?;
            }
        }
        for (node, output) in self.accepts.iter() {
            if let Some(output) = output {
                writeln!(f, "\"{output:?}\" [shape=box];")?;
                writeln!(f, "{} -> \"{output:?}\" [style=\"dashed\"];", node.0)?;
            }
        }
        writeln!(f, "}}")
    }

    pub fn print_mermaid(&self, name: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(name)?;

        writeln!(f, "flowchart LR")?;
        writeln!(
            f,
            "style start fill:#FFFFFF00, stroke:#FFFFFF00\nstart-->{};",
            self.start.0
        )?;
        for (i, node) in self.states.iter().enumerate() {
            writeln!(
                f,
                "{i}@{{shape: {}}}",
                if self.accepts.iter().any(|(node, _)| i == node.0) {
                    "dbl-circ"
                } else {
                    "circ"
                }
            )?;
            for t in node.transitions.iter() {
                let label = format!("{:?}", t.when).replace("\"", "#34;");
                writeln!(f, "{i} -- \"{label}\" --> {}", t.to.0)?;
            }
        }
        for (node, output) in self.accepts.iter() {
            if let Some(output) = output {
                writeln!(
                    f,
                    "{}_{}[{output:?}]@{{shape: rect}}",
                    output.value.0, output.value.1
                )?;
                writeln!(f, "{} .-> {}_{}", node.0, output.value.0, output.value.1)?;
            }
        }
        Ok(())
    }
}
