mod integration;

use integration::pdtk_output;
use proptest::prelude::*;
use serde_json::Value;

// Property test: for any combination of known and unknown flags, the
// right-anchored parser correctly recovers `name` and `size`, and reports
// the right `parse_status` and discarded-token suffix.
proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        .. ProptestConfig::default()
    })]

    #[test]
    fn arrays_define_right_anchored_invariants(
        // 0..3 distinct known flags, in any order.
        knowns in distinct_knowns(),
        // 0..3 unknown flags, each with arity 0..=3.
        unknowns in proptest::collection::vec(unknown_flag(), 0..4),
        name in "[a-zA-Z_][a-zA-Z0-9_]{0,15}",
        size in 0u32..10_000,
    ) {
        // Build the entry: knowns first, then unknowns, then name, size.
        let mut tokens: Vec<String> = Vec::new();
        let mut applied_known: Vec<&'static str> = Vec::new();
        for k in &knowns {
            for t in k.tokens() {
                tokens.push(t.clone());
            }
            applied_known.push(k.name());
        }
        let unknown_start = tokens.len();
        for u in &unknowns {
            for t in u.tokens() {
                tokens.push(t.clone());
            }
        }
        // Disallow generated names that look like a flag — that would be
        // valid PD but ambiguous test setup.
        prop_assume!(!name.starts_with('-'));
        tokens.push(name.clone());
        tokens.push(size.to_string());

        let entry = format!(
            "#N canvas 0 0 100 100 10;\n#X obj 50 50 array define {};\n",
            tokens.join(" ")
        );
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("p.pd");
        std::fs::write(&p, entry).unwrap();

        let out = pdtk_output(&["arrays", p.to_str().unwrap(), "--kind=define", "--json"]);
        let v: Value = serde_json::from_str(&out).unwrap();
        let arrays = v["arrays"].as_array().unwrap();
        prop_assert_eq!(arrays.len(), 1);
        let r = &arrays[0];

        // (1) name and size always correct.
        prop_assert_eq!(r["name"].as_str().unwrap(), &name);
        prop_assert_eq!(r["size"].as_u64().unwrap(), u64::from(size));

        let d = &r["define"];
        let has_unknown = !unknowns.is_empty();
        // (2) parse_status partial iff unknown flags present.
        let status = d["parse_status"].as_str().unwrap();
        if has_unknown {
            prop_assert_eq!(status, "partial");
        } else {
            prop_assert_eq!(status, "clean");
        }

        // (3) Known flags appearing BEFORE the first unknown flag are applied.
        // (Known flags after the first unknown go into the unknown_flag tail.)
        // In our generator, all knowns come before all unknowns, so all
        // knowns are applied.
        let expected_k = applied_known.contains(&"-k");
        prop_assert_eq!(d["k"].as_bool().unwrap(), expected_k);
        let expected_yrange = applied_known.contains(&"-yrange");
        prop_assert_eq!(d["yrange"].is_null(), !expected_yrange);
        let expected_pix = applied_known.contains(&"-pix");
        prop_assert_eq!(d["pix"].is_null(), !expected_pix);

        // (4) discarded_tokens has exactly one unknown_flag record (when any),
        //     containing exactly the unknown-flag suffix.
        let dt = d["discarded_tokens"].as_array().unwrap();
        if has_unknown {
            prop_assert_eq!(dt.len(), 1);
            prop_assert_eq!(dt[0]["reason"].as_str().unwrap(), "unknown_flag");
            let got: Vec<&str> = dt[0]["tokens"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| t.as_str().unwrap())
                .collect();
            let expected: Vec<String> = tokens[unknown_start..tokens.len() - 2]
                .to_vec();
            prop_assert_eq!(got, expected);
        } else {
            prop_assert_eq!(dt.len(), 0);
        }
    }
}

#[derive(Debug, Clone)]
enum Known {
    K,
    Yrange(i32, i32),
    Pix(u32, u32),
}

impl Known {
    fn tokens(&self) -> Vec<String> {
        match self {
            Known::K => vec!["-k".to_string()],
            Known::Yrange(a, b) => vec!["-yrange".to_string(), a.to_string(), b.to_string()],
            Known::Pix(a, b) => vec!["-pix".to_string(), a.to_string(), b.to_string()],
        }
    }
    fn name(&self) -> &'static str {
        match self {
            Known::K => "-k",
            Known::Yrange(..) => "-yrange",
            Known::Pix(..) => "-pix",
        }
    }
}

/// Generate a list of known flags where each kind appears at most once.
fn distinct_knowns() -> impl Strategy<Value = Vec<Known>> {
    (any::<bool>(), any::<bool>(), any::<bool>(), any::<u8>()).prop_flat_map(
        |(use_k, use_yr, use_pix, order)| {
            let mut variants: Vec<BoxedStrategy<Known>> = Vec::new();
            if use_k {
                variants.push(Just(Known::K).boxed());
            }
            if use_yr {
                variants.push(
                    (-100i32..100, -100i32..100)
                        .prop_map(|(a, b)| Known::Yrange(a, b))
                        .boxed(),
                );
            }
            if use_pix {
                variants.push(
                    (10u32..1000, 10u32..1000)
                        .prop_map(|(a, b)| Known::Pix(a, b))
                        .boxed(),
                );
            }
            let n = variants.len();
            let strat: BoxedStrategy<Vec<Known>> = if n == 0 {
                Just(Vec::new()).boxed()
            } else {
                variants
                    .prop_map(move |v| {
                        // simple deterministic shuffle based on `order` byte
                        let mut v = v;
                        let n = v.len();
                        if n > 1 {
                            let r = (order as usize) % n;
                            v.rotate_left(r);
                        }
                        v
                    })
                    .boxed()
            };
            strat
        },
    )
}

#[derive(Debug, Clone)]
struct Unknown {
    flag: String,
    args: Vec<String>,
}

impl Unknown {
    fn tokens(&self) -> Vec<String> {
        let mut t = vec![self.flag.clone()];
        t.extend(self.args.iter().cloned());
        t
    }
}

fn unknown_flag() -> impl Strategy<Value = Unknown> {
    // Unknown flag name: starts with `-`, followed by random letters.
    // Avoid colliding with known flags.
    let flag = "-[a-z]{2,8}".prop_filter("not known", |s: &String| {
        !matches!(s.as_str(), "-k" | "-yrange" | "-pix")
    });
    let args = proptest::collection::vec("[a-zA-Z][a-zA-Z0-9]{0,5}", 0..4);
    (flag, args).prop_map(|(flag, args)| Unknown { flag, args })
}
