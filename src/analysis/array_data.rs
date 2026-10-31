use crate::model::{
    ArrayDecl, Entry, EntryKind, content_without_width_hint, parse_array_data, parse_array_decl,
    takes_array_data,
};

/// The saved contents of one declared array.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayContents {
    /// The declaration this data belongs to.
    pub decl: ArrayDecl,
    /// Index of the declaration entry in the entry slice it was read from.
    pub entry: usize,
    /// Internal depth of the declaration (see `Entry::depth`).
    pub depth: usize,
    /// Canvas the declaration lives in.
    pub canvas_id: Option<usize>,
    /// Values in array-index order, starting at element 0.  Positions no `#A`
    /// record covers are 0.0, and the vector stops after the highest covered
    /// element: it is shorter than `decl.size` when the file saves only part
    /// of the array (PD loads every uncovered element as 0), and empty when
    /// the array saves no data at all.
    pub values: Vec<f64>,
    /// How many `#A` records supplied values.
    pub records: usize,
    /// How many of those records start before the previous one ended, i.e.
    /// overwrite values already written. PD writes long arrays as ascending
    /// non-overlapping chunks, so this is never non-zero in a file PD saved:
    /// it means several arrays' data has piled onto one array (typically an
    /// `#A` block placed after a run of declarations rather than after the
    /// array each record belongs to).
    pub overlapping_records: usize,
    /// Values discarded because they fell beyond the declared size, as PD
    /// does when loading into a fixed-size array.
    pub overflow: usize,
    /// Value tokens that are not numbers (PD reads these as 0).
    pub non_numeric: Vec<String>,
}

impl ArrayContents {
    /// Whether the patch file stores contents for this array.  An array
    /// declared without `-k` (or with an even classic save flag) is not saved,
    /// so PD loads it as all zeros and there is nothing to read here.
    #[must_use]
    pub fn saved(&self) -> bool {
        self.records > 0
    }

    /// Value of element `index`, 0.0 for any element the file does not cover
    /// (which is what PD loads).  `None` past the declared size.
    #[must_use]
    pub fn value(&self, index: usize) -> Option<f64> {
        if index >= self.decl.size {
            return None;
        }
        Some(self.values.get(index).copied().unwrap_or(0.0))
    }
}

/// Where a patch's `#A` records go when PD loads it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArrayDataBinding {
    /// Every declared array in document order, with the contents PD loads into
    /// it — including arrays with no saved data (`records == 0`).
    pub arrays: Vec<ArrayContents>,
    /// Entry indices of `#A` records that precede every object able to take
    /// them, so PD drops them.
    pub unbound: Vec<usize>,
    /// `(record, taker)` entry indices of `#A <onset> …` value records taken by
    /// an object that is not a declared array — a `[table]`, `[text define]`
    /// or `[scalar define]` — so no declared array receives the values.
    /// Records in other forms (the `#A set …` a `text define -k` saves) are
    /// that object's own contents and are not listed.
    pub foreign: Vec<(usize, usize)>,
}

/// Bind every `#A` record in `entries` the way PD's file loader does.
///
/// PD sends an `#A` record to the most recently created object that takes
/// them (see `model::takes_array_data`) — not to the array nearest in the
/// file structure, and not scoped to a canvas. A single array's data may span
/// several `#A` records, each carrying the onset of its first value (PD writes
/// them in chunks), so records are applied at their onset rather than
/// appended.
#[must_use]
pub fn bind_array_data(entries: &[Entry]) -> ArrayDataBinding {
    enum Taker {
        Nothing,
        Array(usize),
        Other(usize),
    }
    let mut binding = ArrayDataBinding::default();
    let mut taker = Taker::Nothing;
    // End (exclusive) of the previously applied record, for overlap detection.
    let mut previous_end = 0usize;
    for (i, e) in entries.iter().enumerate() {
        if e.kind == EntryKind::ArrayData {
            match taker {
                Taker::Array(a) => {
                    let target = &mut binding.arrays[a];
                    if let Some((onset, end)) = apply_record(target, &e.raw) {
                        if onset < previous_end {
                            target.overlapping_records += 1;
                        }
                        previous_end = end;
                    }
                }
                Taker::Other(t) => {
                    if parse_array_data(&e.raw).is_some() {
                        binding.foreign.push((i, t));
                    }
                }
                Taker::Nothing => binding.unbound.push(i),
            }
            continue;
        }
        if !takes_array_data(e) {
            continue;
        }
        previous_end = 0;
        taker = match parse_array_decl(&e.raw) {
            Some(decl) => {
                binding.arrays.push(ArrayContents {
                    decl,
                    entry: i,
                    depth: e.depth,
                    canvas_id: e.canvas_id,
                    values: Vec::new(),
                    records: 0,
                    overlapping_records: 0,
                    overflow: 0,
                    non_numeric: Vec::new(),
                });
                Taker::Array(binding.arrays.len() - 1)
            }
            None => Taker::Other(i),
        };
    }
    binding
}

/// Read the saved contents of every array declared in `entries`, in document
/// order (see `bind_array_data`).
#[must_use]
pub fn array_contents(entries: &[Entry]) -> Vec<ArrayContents> {
    bind_array_data(entries).arrays
}

/// Warnings for `#A` records PD will not load the way the file reads: records
/// nothing takes, value records taken by a non-array object, records that
/// overwrite each other because they were written as a block after several
/// declarations, and values past an array's declared size. Returns
/// human-readable messages (empty = no problems found).
#[must_use]
pub fn array_data_warnings(entries: &[Entry]) -> Vec<String> {
    let binding = bind_array_data(entries);
    let mut msgs = Vec::new();
    for &i in &binding.unbound {
        msgs.push(format!(
            "entry {i}: #A array data not attached to an array definition: {}",
            entries[i].raw.trim()
        ));
    }
    for &(i, t) in &binding.foreign {
        let tokens: Vec<&str> = content_without_width_hint(&entries[t].raw)
            .split_whitespace()
            .collect();
        // Show an object as its box text (`table tt 3`), anything else whole.
        let skip = if entries[t].kind == EntryKind::Obj {
            4
        } else {
            0
        };
        let taker = tokens.get(skip..).unwrap_or_default().join(" ");
        msgs.push(format!(
            "entry {i}: #A array data goes to [{taker}] (entry {t}), the most recent object \
             that takes #A records, not to an array — put each record directly after the \
             array it belongs to"
        ));
    }
    for c in &binding.arrays {
        let user_depth = c.depth.saturating_sub(1);
        let name = if c.decl.name.is_empty() {
            "(anonymous)".to_string()
        } else {
            format!("'{}'", c.decl.name)
        };
        if c.overlapping_records > 0 {
            msgs.push(format!(
                "depth {user_depth}: {} of {} #A record(s) overwrite array {name} \
                 (each starts before the previous ended) — PD sends each #A record \
                 to the most recent array, table, text or scalar define before it, \
                 so put each record directly after the array it belongs to",
                c.overlapping_records, c.records
            ));
        }
        if c.overflow > 0 {
            msgs.push(format!(
                "depth {user_depth}: array {name} has size {} but its #A data supplies \
                 {} more value(s), which PD discards",
                c.decl.size, c.overflow
            ));
        }
    }
    msgs
}

/// Apply one `#A` record's values into `target` at the record's onset.
/// Returns the record's `(onset, end)` range, or `None` if it is malformed.
fn apply_record(target: &mut ArrayContents, raw: &str) -> Option<(usize, usize)> {
    let (onset, tokens) = parse_array_data(raw)?;
    target.records += 1;
    for (offset, tok) in tokens.iter().enumerate() {
        let index = onset + offset;
        if index >= target.decl.size {
            target.overflow += 1;
            continue;
        }
        let value = match tok.parse::<f64>() {
            Ok(v) if v.is_finite() => v,
            _ => {
                target.non_numeric.push(tok.clone());
                0.0
            }
        };
        if index >= target.values.len() {
            target.values.resize(index + 1, 0.0);
        }
        target.values[index] = value;
    }
    Some((onset, onset + tokens.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn contents(src: &str) -> Vec<ArrayContents> {
        array_contents(&parse(src).unwrap().entries)
    }

    #[test]
    fn reads_classic_array_data() {
        let c = contents(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#N canvas 0 50 450 250 (subpatch) 0;\n",
            "#X array wave 4 float 3;\n",
            "#A 0 1 2 3 4;\n",
            "#X restore 50 50 graph;\n",
        ));
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].decl.name, "wave");
        assert_eq!(c[0].values, vec![1.0, 2.0, 3.0, 4.0]);
        assert!(c[0].saved());
    }

    #[test]
    fn joins_chunked_records_at_their_onset() {
        let c = contents(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#X obj 10 10 array define -k data 6;\n",
            "#A 0 1 2 3;\n",
            "#A 3 4 5 6;\n",
        ));
        assert_eq!(c[0].values, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(c[0].records, 2);
    }

    #[test]
    fn data_binds_to_the_most_recent_declaration() {
        let c = contents(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#X obj 10 10 array define -k first 2;\n",
            "#A 0 1 2;\n",
            "#X obj 10 50 array define -k second 2;\n",
            "#A 0 3 4;\n",
        ));
        assert_eq!(c[0].values, vec![1.0, 2.0]);
        assert_eq!(c[1].values, vec![3.0, 4.0]);
    }

    #[test]
    fn unsaved_array_reports_no_data() {
        let c = contents(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#X obj 10 10 array define plain 4;\n",
        ));
        assert!(!c[0].saved());
        assert!(c[0].values.is_empty());
        assert_eq!(c[0].value(2), Some(0.0));
        assert_eq!(c[0].value(4), None);
    }

    #[test]
    fn discards_values_beyond_the_declared_size() {
        let c = contents(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#X obj 10 10 array define -k small 2;\n",
            "#A 0 1 2 3 4;\n",
        ));
        assert_eq!(c[0].values, vec![1.0, 2.0]);
        assert_eq!(c[0].overflow, 2);
    }

    #[test]
    fn gaps_between_records_read_as_zero() {
        let c = contents(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#X obj 10 10 array define -k sparse 5;\n",
            "#A 3 7 8;\n",
        ));
        assert_eq!(c[0].values, vec![0.0, 0.0, 0.0, 7.0, 8.0]);
    }

    #[test]
    fn non_numeric_tokens_read_as_zero_and_are_reported() {
        let c = contents(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#X obj 10 10 array define -k odd 4;\n",
            "#A 0 1 bang 3 inf;\n",
        ));
        // PD reads `inf` as a symbol too, so it loads as 0 (checked against
        // Pd 0.54), even though Rust parses it as a float.
        assert_eq!(c[0].values, vec![1.0, 0.0, 3.0, 0.0]);
        assert_eq!(
            c[0].non_numeric,
            vec!["bang".to_string(), "inf".to_string()]
        );
    }

    #[test]
    fn chunked_records_do_not_count_as_overlapping() {
        let c = contents(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#X obj 10 10 array define -k chunks 6;\n",
            "#A 0 1 2 3;\n",
            "#A 3 4 5 6;\n",
        ));
        assert_eq!(c[0].overlapping_records, 0);
    }

    #[test]
    fn a_block_of_records_after_several_declarations_all_lands_on_the_last() {
        // PD binds every `#A` to the most recently declared array, so data
        // written as a block after a run of declarations piles onto one array
        // (verified against Pd 0.54).
        let c = contents(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#X obj 10 10 array define -k first 2;\n",
            "#X obj 10 50 array define -k second 2;\n",
            "#A 0 11 12;\n",
            "#A 0 21 22;\n",
        ));
        assert!(!c[0].saved(), "first array gets nothing");
        assert_eq!(c[1].values, vec![21.0, 22.0], "last record wins");
        assert_eq!(c[1].records, 2);
        assert_eq!(c[1].overlapping_records, 1);
    }

    #[test]
    fn detached_data_before_any_declaration_is_unbound() {
        let entries = parse(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#A 0 1 2 3;\n",
            "#X obj 10 10 array define -k later 2;\n",
        ))
        .unwrap()
        .entries;
        let b = bind_array_data(&entries);
        assert_eq!(b.arrays.len(), 1);
        assert!(!b.arrays[0].saved());
        assert_eq!(b.unbound, vec![1]);
    }

    #[test]
    fn array_define_without_a_size_takes_its_own_data() {
        // PD gives `[array define -k second]` 100 elements and sends it the
        // records that follow; they must not pile onto `first` (checked
        // against Pd 0.54).
        let c = contents(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#X obj 10 10 array define -k first 3;\n",
            "#A 0 1 2 3;\n",
            "#X obj 10 50 array define -k second;\n",
            "#A 0 7 8 9 10;\n",
        ));
        assert_eq!(c[0].values, vec![1.0, 2.0, 3.0]);
        assert_eq!(c[0].overflow, 0);
        assert_eq!(c[1].decl.name, "second");
        assert_eq!(c[1].decl.size, 100);
        assert_eq!(c[1].values, vec![7.0, 8.0, 9.0, 10.0]);
    }

    #[test]
    fn records_after_a_table_or_text_go_to_it_not_the_array() {
        // `[table]` and `[text define]` take `#A` records too, so the array
        // declared before them loads as zeros (checked against Pd 0.54).
        for taker in ["table tt 3", "text define t"] {
            let entries = parse(&format!(
                "#N canvas 0 22 450 300 12;\n\
                 #X obj 10 10 array define -k a 3;\n\
                 #X obj 10 40 {taker};\n\
                 #A 0 1 2 3;\n"
            ))
            .unwrap()
            .entries;
            let b = bind_array_data(&entries);
            assert!(!b.arrays[0].saved(), "{taker}");
            assert_eq!(b.foreign, vec![(3, 2)], "{taker}");
            assert!(b.unbound.is_empty(), "{taker}");
        }
    }

    #[test]
    fn a_text_define_keeps_its_own_saved_contents() {
        let entries = parse(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#X obj 10 10 text define -k notes;\n",
            "#A set 60 62 64 \\; 65 67 \\;;\n",
        ))
        .unwrap()
        .entries;
        assert!(array_data_warnings(&entries).is_empty());
    }
}
