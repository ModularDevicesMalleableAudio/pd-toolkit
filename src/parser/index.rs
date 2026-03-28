use crate::model::{Entry, EntryKind};

use super::classify::classify_entry;

pub fn build_entries(raw_entries: &[String]) -> Vec<Entry> {
    let mut entries: Vec<Entry> = raw_entries
        .iter()
        .map(|raw| Entry {
            raw: raw.clone(),
            kind: classify_entry(raw),
            depth: 0,
            object_index: None,
        })
        .collect();

    assign_depth_and_indices(&mut entries);
    entries
}

pub fn assign_depth_and_indices(entries: &mut [Entry]) {
    let mut depth: usize = 0;
    // One object counter per open canvas (resets when a new canvas opens)
    let mut canvas_counters: Vec<usize> = Vec::new();

    for entry in entries.iter_mut() {
        match entry.kind {
            EntryKind::CanvasOpen => {
                entry.depth = depth;
                entry.object_index = None;

                depth += 1;
                canvas_counters.push(0);
            }
            EntryKind::Restore => {
                // Close current canvas first
                depth = depth.saturating_sub(1);
                canvas_counters.pop();

                // Restore box is an object in parent canvas
                entry.depth = depth;
                if let Some(parent_counter) = canvas_counters.last_mut() {
                    entry.object_index = Some(*parent_counter);
                    *parent_counter += 1;
                } else {
                    entry.object_index = None;
                }
            }
            EntryKind::Obj
            | EntryKind::Msg
            | EntryKind::Text
            | EntryKind::FloatAtom
            | EntryKind::SymbolAtom => {
                entry.depth = depth;
                if let Some(counter) = canvas_counters.last_mut() {
                    entry.object_index = Some(*counter);
                    *counter += 1;
                } else {
                    entry.object_index = None;
                }
            }
            _ => {
                entry.depth = depth;
                entry.object_index = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_entries(input: &str) -> Vec<Entry> {
        let tokenized = crate::parser::tokenize_entries(input);
        build_entries(&tokenized.entries)
    }

    fn object_indices_at_depth(entries: &[Entry], depth: usize) -> Vec<usize> {
        entries
            .iter()
            .filter(|e| e.depth == depth)
            .filter_map(|e| e.object_index)
            .collect()
    }

    #[test]
    fn index_flat_patch_sequential_indices() {
        let input = "#N canvas 0 0 100 100 10;\n#X obj 10 10 f;\n#X msg 10 20 bang;\n#X text 10 30 hi;\n#X connect 0 0 1 0;";
        let entries = parse_entries(input);

        assert_eq!(object_indices_at_depth(&entries, 1), vec![0, 1, 2]);
    }

    #[test]
    fn index_standalone_declare_skipped() {
        let input = include_str!("../../tests/fixtures/handcrafted/with_declare.pd");
        let entries = parse_entries(input);

        // depth 1 because top-level #N canvas increments depth
        let objects = object_indices_at_depth(&entries, 1);
        assert_eq!(objects.len(), 10);
        assert_eq!(objects, (0..10).collect::<Vec<_>>());

        let declare_entry = entries
            .iter()
            .find(|e| e.raw.starts_with("#X declare "))
            .expect("missing #X declare entry");
        assert_eq!(declare_entry.object_index, None);
    }

    #[test]
    fn index_width_hint_skipped() {
        let input = include_str!("../../tests/fixtures/handcrafted/with_width_hint.pd");
        let entries = parse_entries(input);

        let top_objects = entries
            .iter()
            .filter(|e| e.depth == 1)
            .filter_map(|e| {
                e.object_index.map(|idx| {
                    (
                        idx,
                        e.raw.lines().next().unwrap_or_default().to_string(),
                        e.kind.clone(),
                    )
                })
            })
            .collect::<Vec<_>>();

        assert_eq!(top_objects.len(), 2);
        assert!(top_objects[0].1.starts_with("#X restore"));
        assert!(top_objects[1].1.starts_with("#X obj"));

        let width_hint = entries
            .iter()
            .find(|e| e.raw.trim() == "#X f 38;")
            .expect("missing width hint");
        assert_eq!(width_hint.object_index, None);
    }

    #[test]
    fn index_restore_at_parent_depth() {
        let input = include_str!("../../tests/fixtures/handcrafted/nested_subpatch.pd");
        let entries = parse_entries(input);

        // top-level objects are at depth 1
        let top_objects = entries
            .iter()
            .filter(|e| e.depth == 1)
            .filter_map(|e| e.object_index.map(|idx| (idx, e.raw.clone())))
            .collect::<Vec<_>>();

        assert_eq!(top_objects.len(), 3);
        assert!(top_objects[0].1.starts_with("#X obj 50 50 inlet;"));
        assert!(top_objects[1].1.starts_with("#X restore 50 100 pd my_sub;"));
        assert!(top_objects[2].1.starts_with("#X obj 50 150 outlet;"));
    }

    #[test]
    fn index_restore_graph_at_parent_depth() {
        let input = include_str!("../../tests/fixtures/handcrafted/with_graph.pd");
        let entries = parse_entries(input);

        let top_objects = entries
            .iter()
            .filter(|e| e.depth == 1)
            .filter_map(|e| e.object_index.map(|idx| (idx, e.raw.clone())))
            .collect::<Vec<_>>();

        assert_eq!(top_objects.len(), 4);
        assert!(top_objects[1].1.starts_with("#X restore 50 100 graph;"));
    }

    #[test]
    fn index_deep_nesting_correct_per_depth() {
        let input = include_str!("../../tests/fixtures/handcrafted/deep_subpatch.pd");
        let entries = parse_entries(input);

        // depths are shifted by +1 because the root canvas opens depth 1
        assert_eq!(object_indices_at_depth(&entries, 1), vec![0, 1, 2]);
        assert_eq!(object_indices_at_depth(&entries, 2), vec![0, 1, 2]);
        assert_eq!(object_indices_at_depth(&entries, 3), vec![0, 1, 2]);
        assert_eq!(object_indices_at_depth(&entries, 4), vec![0, 1, 2]);
    }

    #[test]
    fn index_c_entry_skipped() {
        let input = include_str!("../../tests/fixtures/handcrafted/with_c_entry.pd");
        let entries = parse_entries(input);

        let c_entry = entries
            .iter()
            .find(|e| e.raw.trim() == "#C restore;")
            .expect("missing #C restore entry");
        assert_eq!(c_entry.object_index, None);

        // Should still have sequential object indices at top depth
        let top_objects = object_indices_at_depth(&entries, 1);
        assert_eq!(top_objects, (0..7).collect::<Vec<_>>());
    }

    #[test]
    fn index_multiple_subpatches_independent() {
        let input = include_str!("../../tests/fixtures/handcrafted/multiple_subpatches.pd");
        let entries = parse_entries(input);

        // Top-level should include two restore objects in order
        let top_objects = entries
            .iter()
            .filter(|e| e.depth == 1)
            .filter_map(|e| e.object_index.map(|idx| (idx, e.raw.clone())))
            .collect::<Vec<_>>();

        assert_eq!(top_objects.len(), 5);
        assert!(top_objects[1].1.contains("pd sub_a"));
        assert!(top_objects[2].1.contains("pd sub_b"));

        // Depth 2 objects from both subpatches should each have local 0..2 indexing,
        // appearing as 0,1,2,3,4,5 across textual order only if treated globally.
        // We verify local reset by checking both inlet entries are index 0 at depth 2.
        let depth2_inlets = entries
            .iter()
            .filter(|e| e.depth == 2)
            .filter(|e| e.raw.contains(" inlet;"))
            .map(|e| e.object_index)
            .collect::<Vec<_>>();

        assert_eq!(depth2_inlets, vec![Some(0), Some(0)]);
    }

    #[test]
    fn index_text_entries_get_indices() {
        let input = "#N canvas 0 0 100 100 10;\n#X text 10 10 hello;\n#X obj 10 20 print;\n#X connect 0 0 1 0;";
        let entries = parse_entries(input);

        let objs = entries
            .iter()
            .filter(|e| e.depth == 1)
            .filter_map(|e| e.object_index)
            .collect::<Vec<_>>();
        assert_eq!(objs, vec![0, 1]);

        let text_entry = entries
            .iter()
            .find(|e| e.raw.starts_with("#X text"))
            .expect("missing text entry");
        assert_eq!(text_entry.object_index, Some(0));
    }
}
