use crate::model::{
    ArrayDecl, ArrayDeclParse, Entry, EntryKind, parse_array_data, parse_array_decl,
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

/// Read the saved contents of every array declared in `entries`.
///
/// An `#A` record binds to the most recently declared array, exactly as PD's
/// file loader does — not to the array nearest in the file structure, and not
/// scoped to a canvas. A single array's data may span several `#A` records,
/// each carrying the onset of its first value (PD writes them in chunks), so
/// records are applied at their onset rather than appended.
///
/// Every declaration is returned in document order, including arrays with no
/// saved data (`records == 0`); an `#A` record that precedes any declaration
/// is ignored (`validate` reports those as detached).
#[must_use]
pub fn array_contents(entries: &[Entry]) -> Vec<ArrayContents> {
    let mut out: Vec<ArrayContents> = Vec::new();
    // End (exclusive) of the previously applied record, for overlap detection.
    let mut previous_end = 0usize;
    for (i, e) in entries.iter().enumerate() {
        if e.kind == EntryKind::ArrayData {
            if let Some(target) = out.last_mut()
                && let Some((onset, end)) = apply_record(target, &e.raw)
            {
                if onset < previous_end {
                    target.overlapping_records += 1;
                }
                previous_end = end;
            }
            continue;
        }
        if let ArrayDeclParse::Decl(decl) = parse_array_decl(&e.raw) {
            out.push(ArrayContents {
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
            previous_end = 0;
        }
    }
    out
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
            "#X obj 10 10 array define -k odd 3;\n",
            "#A 0 1 bang 3;\n",
        ));
        assert_eq!(c[0].values, vec![1.0, 0.0, 3.0]);
        assert_eq!(c[0].non_numeric, vec!["bang".to_string()]);
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
    fn detached_data_before_any_declaration_is_ignored() {
        let c = contents(concat!(
            "#N canvas 0 22 450 300 12;\n",
            "#A 0 1 2 3;\n",
            "#X obj 10 10 array define -k later 2;\n",
        ));
        assert_eq!(c.len(), 1);
        assert!(!c[0].saved());
    }
}
