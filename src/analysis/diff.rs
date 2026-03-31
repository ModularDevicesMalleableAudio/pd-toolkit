use crate::model::{Entry, Patch};
use similar::{Algorithm, DiffOp, capture_diff_slices};
use std::collections::HashMap;

/// Text key for LCS matching (class+args, always coords-stripped).
fn match_key(e: &Entry) -> String {
    let class = e.class();
    let args = e.args();
    if args.is_empty() {
        class.to_string()
    } else {
        format!("{} {}", class, args.join(" "))
    }
}

/// Returns true when the only difference between two matched entries is the X/Y coordinates.
fn only_coords_differ(a: &Entry, b: &Entry) -> bool {
    match_key(a) == match_key(b) && a.raw != b.raw
}

/// Build an index mapping a_idx → b_idx from the Equal regions of a slice diff.
fn build_index_mapping(a_keys: &[String], b_keys: &[String]) -> HashMap<usize, usize> {
    let ops = capture_diff_slices(Algorithm::Patience, a_keys, b_keys);
    let mut map = HashMap::new();
    for op in ops {
        if let DiffOp::Equal {
            old_index,
            new_index,
            len,
        } = op
        {
            for i in 0..len {
                map.insert(old_index + i, new_index + i);
            }
        }
    }
    map
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct ObjectChange {
    pub depth: usize,
    pub index: usize,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_text: Option<String>,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct ConnectionChange {
    pub depth: usize,
    pub src: usize,
    pub src_outlet: usize,
    pub dst: usize,
    pub dst_inlet: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct DiffResult {
    pub objects_added: Vec<ObjectChange>,
    pub objects_removed: Vec<ObjectChange>,
    pub objects_modified: Vec<ObjectChange>,
    pub connections_added: Vec<ConnectionChange>,
    pub connections_removed: Vec<ConnectionChange>,
}

impl DiffResult {
    pub fn is_empty(&self) -> bool {
        self.objects_added.is_empty()
            && self.objects_removed.is_empty()
            && self.objects_modified.is_empty()
            && self.connections_added.is_empty()
            && self.connections_removed.is_empty()
    }
}

/// Compute structural diff between two patches.
pub fn diff_patches(a: &Patch, b: &Patch, ignore_coords: bool) -> DiffResult {
    let max_depth = a.max_depth().max(b.max_depth());

    let mut objects_added = Vec::new();
    let mut objects_removed = Vec::new();
    let mut objects_modified = Vec::new();
    let mut connections_added = Vec::new();
    let mut connections_removed = Vec::new();

    for depth in 0..=max_depth {
        let internal = depth + 1;

        let a_objs: Vec<&Entry> = a
            .entries
            .iter()
            .filter(|e| e.depth == internal && e.object_index.is_some())
            .collect();
        let b_objs: Vec<&Entry> = b
            .entries
            .iter()
            .filter(|e| e.depth == internal && e.object_index.is_some())
            .collect();

        let a_keys: Vec<String> = a_objs.iter().map(|e| match_key(e)).collect();
        let b_keys: Vec<String> = b_objs.iter().map(|e| match_key(e)).collect();

        let ops = capture_diff_slices(Algorithm::Patience, &a_keys, &b_keys);

        // a_idx → b_idx for Equal regions
        let a_to_b = build_index_mapping(&a_keys, &b_keys);

        for op in &ops {
            match *op {
                DiffOp::Equal {
                    old_index,
                    new_index,
                    len,
                } => {
                    for i in 0..len {
                        let ea = a_objs[old_index + i];
                        let eb = b_objs[new_index + i];
                        if ea.raw != eb.raw {
                            let coord_only = only_coords_differ(ea, eb);
                            if coord_only && ignore_coords {
                                // suppress
                            } else {
                                objects_modified.push(ObjectChange {
                                    depth,
                                    index: old_index + i,
                                    text: String::new(),
                                    old_text: Some(ea.raw.trim().to_string()),
                                    new_text: Some(eb.raw.trim().to_string()),
                                });
                            }
                        }
                    }
                }
                DiffOp::Delete {
                    old_index, old_len, ..
                } => {
                    for i in 0..old_len {
                        let e = a_objs[old_index + i];
                        objects_removed.push(ObjectChange {
                            depth,
                            index: old_index + i,
                            text: e.raw.trim().to_string(),
                            old_text: None,
                            new_text: None,
                        });
                    }
                }
                DiffOp::Insert {
                    new_index, new_len, ..
                } => {
                    for i in 0..new_len {
                        let e = b_objs[new_index + i];
                        objects_added.push(ObjectChange {
                            depth,
                            index: new_index + i,
                            text: e.raw.trim().to_string(),
                            old_text: None,
                            new_text: None,
                        });
                    }
                }
                DiffOp::Replace {
                    old_index,
                    old_len,
                    new_index,
                    new_len,
                } => {
                    // 1:1 replacements are reported as "modified"; N:M as remove+add
                    if old_len == new_len {
                        for i in 0..old_len {
                            let ea = a_objs[old_index + i];
                            let eb = b_objs[new_index + i];
                            let coord_only = only_coords_differ(ea, eb);
                            if coord_only && ignore_coords {
                                // suppress
                            } else {
                                objects_modified.push(ObjectChange {
                                    depth,
                                    index: old_index + i,
                                    text: String::new(),
                                    old_text: Some(ea.raw.trim().to_string()),
                                    new_text: Some(eb.raw.trim().to_string()),
                                });
                            }
                        }
                    } else {
                        for i in 0..old_len {
                            let e = a_objs[old_index + i];
                            objects_removed.push(ObjectChange {
                                depth,
                                index: old_index + i,
                                text: e.raw.trim().to_string(),
                                old_text: None,
                                new_text: None,
                            });
                        }
                        for i in 0..new_len {
                            let e = b_objs[new_index + i];
                            objects_added.push(ObjectChange {
                                depth,
                                index: new_index + i,
                                text: e.raw.trim().to_string(),
                                old_text: None,
                                new_text: None,
                            });
                        }
                    }
                }
            }
        }

        // Connection diff — translate a's connections through the index mapping
        let a_conns = a.connections_at_depth(depth);
        let b_conns = b.connections_at_depth(depth);

        use std::collections::HashSet;
        let b_conn_set: HashSet<(usize, usize, usize, usize)> = b_conns
            .iter()
            .map(|c| (c.src, c.src_outlet, c.dst, c.dst_inlet))
            .collect();

        // Translated a connections (where both endpoints have a mapping)
        let mut translated_a: HashSet<(usize, usize, usize, usize)> = HashSet::new();
        for c in &a_conns {
            if let (Some(&src_b), Some(&dst_b)) = (a_to_b.get(&c.src), a_to_b.get(&c.dst)) {
                translated_a.insert((src_b, c.src_outlet, dst_b, c.dst_inlet));
            }
        }

        for &(src, outlet, dst, inlet) in b_conn_set.difference(&translated_a) {
            connections_added.push(ConnectionChange {
                depth,
                src,
                src_outlet: outlet,
                dst,
                dst_inlet: inlet,
            });
        }
        for &(src, outlet, dst, inlet) in translated_a.difference(&b_conn_set) {
            connections_removed.push(ConnectionChange {
                depth,
                src,
                src_outlet: outlet,
                dst,
                dst_inlet: inlet,
            });
        }
    }

    DiffResult {
        objects_added,
        objects_removed,
        objects_modified,
        connections_added,
        connections_removed,
    }
}
