/// The data type carried on a patch cord outlet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutletType {
    Bang,
    Float,
    Symbol,
    List,
    Signal,
    Unknown,
}

/// Returns the outlet type list for a known PD object class, or `None` for
/// objects not in the built-in table.
///
/// For `trigger` (`t`), the argument list is parsed: each letter maps to an
/// outlet type (b=Bang, f=Float, s=Symbol, l=List, a=anything→List,
/// p=pointer→Unknown).
#[must_use]
pub fn outlet_types(class: &str, args: &[&str]) -> Option<Vec<OutletType>> {
    match class {
        // Trigger: one outlet per arg, left→right matches outlet 0, 1, 2, …
        "t" | "trigger" => {
            if args.is_empty() {
                return None;
            }
            let types = args
                .iter()
                .map(|a| match *a {
                    "b" => OutletType::Bang,
                    "f" => OutletType::Float,
                    "s" => OutletType::Symbol,
                    "l" | "a" => OutletType::List,
                    _ => OutletType::Unknown,
                })
                .collect();
            Some(types)
        }

        // Arithmetic and math
        "+" | "-" | "*" | "/" | "%" | "pow" | "log" | "sqrt" | "abs" => {
            Some(vec![OutletType::Float])
        }
        ">" | "<" | ">=" | "<=" | "==" | "!=" => Some(vec![OutletType::Float]),
        "max" | "min" | "clip" => Some(vec![OutletType::Float]),
        "int" | "i" | "floor" | "ceil" | "rint" => Some(vec![OutletType::Float]),
        "random" => Some(vec![OutletType::Float]),
        "wrap" => Some(vec![OutletType::Float]),

        // Float box
        "f" | "float" | "nb" => Some(vec![OutletType::Float]),

        // Int box
        // Symbol box
        "symbol" => Some(vec![OutletType::Symbol]),

        // Message-passing
        "bang" | "b" => Some(vec![OutletType::Bang]),
        "loadbang" => Some(vec![OutletType::Bang]),
        "metro" => Some(vec![OutletType::Bang]),
        "delay" | "pipe" => Some(vec![OutletType::Bang]),
        "timer" => Some(vec![OutletType::Float]),
        "toggle" => Some(vec![OutletType::Float]),

        // Control
        "spigot" | "gate" => Some(vec![OutletType::Unknown]),
        "moses" => Some(vec![OutletType::Float, OutletType::Float]),
        "select" | "sel" => {
            // One bang outlet per selector arg + one reject outlet
            if args.is_empty() {
                Some(vec![OutletType::Bang, OutletType::Unknown])
            } else {
                let mut v: Vec<OutletType> = args.iter().map(|_| OutletType::Bang).collect();
                v.push(OutletType::Unknown); // reject
                Some(v)
            }
        }
        "route" => {
            if args.is_empty() {
                Some(vec![OutletType::List, OutletType::List])
            } else {
                let mut v: Vec<OutletType> = args.iter().map(|_| OutletType::List).collect();
                v.push(OutletType::List); // reject
                Some(v)
            }
        }

        // Packaging / unpacking
        "pack" => Some(vec![OutletType::List]),
        "unpack" => {
            if args.is_empty() {
                Some(vec![OutletType::Unknown])
            } else {
                let types = args
                    .iter()
                    .map(|a| match *a {
                        "f" => OutletType::Float,
                        "s" => OutletType::Symbol,
                        _ => OutletType::Unknown,
                    })
                    .collect();
                Some(types)
            }
        }
        "list" => Some(vec![OutletType::List, OutletType::List]),
        "bag" => Some(vec![OutletType::List]),

        // Counter / sequence
        "counter" => Some(vec![OutletType::Float]),
        "mod" | "modulo" => Some(vec![OutletType::Float]),

        // Send / receive — send has no outlets, receive has one unknown outlet
        "s" | "send" => Some(vec![]),
        "r" | "receive" => Some(vec![OutletType::Unknown]),
        "s~" => Some(vec![]),
        "r~" => Some(vec![OutletType::Signal]),
        "throw~" => Some(vec![]),
        "catch~" => Some(vec![OutletType::Signal]),

        // Signal
        "osc~" | "phasor~" | "saw~" | "square~" | "tri~" => Some(vec![OutletType::Signal]),
        "sig~" => Some(vec![OutletType::Signal]),
        "noise~" => Some(vec![OutletType::Signal]),
        "dac~" | "adc~" => Some(vec![]),
        "line~" => Some(vec![OutletType::Signal]),
        "vline~" => Some(vec![OutletType::Signal]),
        "snapshot~" => Some(vec![OutletType::Float]),
        "samphold~" => Some(vec![OutletType::Signal]),
        "samplerate~" => Some(vec![OutletType::Float]),
        "tabread~" | "tabosc4~" => Some(vec![OutletType::Signal]),
        "tabwrite~" => Some(vec![]),
        "fft~" | "ifft~" => Some(vec![OutletType::Signal, OutletType::Signal]),
        "*~" | "+~" | "-~" | "/~" => Some(vec![OutletType::Signal]),
        "hip~" | "lop~" | "bp~" | "vcf~" | "rzero~" | "rpole~" => Some(vec![OutletType::Signal]),
        "delwrite~" | "delread~" | "vd~" => Some(vec![OutletType::Signal]),
        "env~" => Some(vec![OutletType::Float]),

        // Tables / arrays
        "tabread" => Some(vec![OutletType::Float]),
        "tabwrite" => Some(vec![]),
        "table" => Some(vec![]),

        // Math compound
        "line" => Some(vec![OutletType::Float, OutletType::Bang]),
        "vsl" | "hsl" | "vradio" | "hradio" | "nbx" | "tgl" | "bng" => {
            Some(vec![OutletType::Float])
        }
        "vu" => Some(vec![OutletType::Float, OutletType::Float]),

        // Misc
        "print" => Some(vec![]),
        "text" => Some(vec![]),
        "message" | "msg" => Some(vec![OutletType::Unknown]),
        "inlet" => Some(vec![OutletType::Unknown]),
        "inlet~" => Some(vec![OutletType::Signal]),
        "outlet" | "outlet~" => Some(vec![]),

        _ => None,
    }
}

/// Returns the number of outlets for a known object, or `None` for unknowns.
#[must_use]
pub fn outlet_count(class: &str, args: &[&str]) -> Option<usize> {
    outlet_types(class, args).map(|v| v.len())
}

/// Returns the number of inlets for a known object class, or `None` when the
/// class is unknown (an external/abstraction could have any number of inlets).
///
/// This table is deliberately conservative: it only lists classes whose inlet
/// count is fixed and well-known in vanilla Pd, and never reports fewer inlets
/// than an object actually has (so a valid patch never produces a false
/// "inlet out of range" warning).
#[must_use]
pub fn inlet_count(class: &str, args: &[&str]) -> Option<usize> {
    let n = match class {
        // Sources with no inlets.
        "loadbang" | "key" | "keyname" | "notein" | "bendin" | "pgmin" | "adc~" => 0,

        // A named receive has no inlets; a bare receive is settable in recent
        // Pd (it gains an inlet), so report unknown rather than undercount.
        "receive" | "r" | "receive~" | "r~" => {
            if args.is_empty() {
                return None;
            }
            0
        }

        // A named send has one inlet; a bare [send]/[s] gains a right inlet
        // that sets the destination.
        "send" | "s" => {
            if args.is_empty() {
                2
            } else {
                1
            }
        }

        // Single-inlet objects (fixed arity).
        "print" | "bang" | "b" | "trigger" | "t" | "unpack" | "send~" | "s~" | "throw~" | "tgl"
        | "toggle" | "bng" => 1,

        // pack: one inlet per element (min 2 when unspecified: `pack f f`).
        "pack" => {
            if args.is_empty() {
                2
            } else {
                args.len()
            }
        }

        // Two-inlet objects (fixed arity). Variable-inlet classes (pipe, line,
        // clip, dac~, symbol, …) are intentionally omitted to avoid ever
        // reporting fewer inlets than an object really has.
        "metro" | "delay" | "del" | "moses" | "swap" | "spigot" | "+" | "-" | "*" | "/" | "%"
        | "pow" | "min" | "max" | "mod" | "div" | ">" | "<" | ">=" | "<=" | "==" | "!=" | "+~"
        | "-~" | "*~" | "/~" | "osc~" | "phasor~" => 2,

        _ => return None,
    };
    Some(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_loadbang_has_bang_outlet() {
        let types = outlet_types("loadbang", &[]).unwrap();
        assert_eq!(types, vec![OutletType::Bang]);
    }

    #[test]
    fn signatures_float_box_has_float_outlet() {
        let types = outlet_types("f", &[]).unwrap();
        assert_eq!(types, vec![OutletType::Float]);
    }

    #[test]
    fn signatures_trigger_parses_args_b_f_s_l_a() {
        let args = &["b", "f", "s", "l", "a"];
        let types = outlet_types("t", args).unwrap();
        assert_eq!(
            types,
            vec![
                OutletType::Bang,
                OutletType::Float,
                OutletType::Symbol,
                OutletType::List,
                OutletType::List,
            ]
        );
    }

    #[test]
    fn signatures_osc_tilde_is_signal() {
        let types = outlet_types("osc~", &[]).unwrap();
        assert_eq!(types, vec![OutletType::Signal]);
    }

    #[test]
    fn signatures_send_has_no_outlets() {
        let types = outlet_types("s", &["my_send"]).unwrap();
        assert!(types.is_empty());
    }

    #[test]
    fn signatures_unknown_object_returns_unknown() {
        assert!(outlet_types("some_external_object", &[]).is_none());
    }

    #[test]
    fn signatures_metro_has_bang_outlet() {
        let types = outlet_types("metro", &["500"]).unwrap();
        assert_eq!(types, vec![OutletType::Bang]);
    }

    #[test]
    fn inlet_counts_never_undercount_common_classes() {
        // Regression guards: these must not report fewer inlets than the object
        // really has, or valid patches produce false warnings.
        assert_eq!(inlet_count("spigot", &[]), Some(2));
        assert_eq!(inlet_count("print", &[]), Some(1));
        assert_eq!(inlet_count("metro", &["500"]), Some(2));
        assert_eq!(inlet_count("loadbang", &[]), Some(0));
        // A bare [send]/[s] has a right inlet that sets the destination.
        assert_eq!(inlet_count("send", &[]), Some(2));
        assert_eq!(inlet_count("s", &[]), Some(2));
        assert_eq!(inlet_count("send", &["dest"]), Some(1));
        // A named receive has no inlets; a bare one is settable (unknown).
        assert_eq!(inlet_count("receive", &["src"]), Some(0));
        assert_eq!(inlet_count("r", &[]), None);
        assert_eq!(inlet_count("pack", &["f", "f", "f"]), Some(3));
        assert_eq!(inlet_count("pack", &[]), Some(2));
        // Unknown / variable-arity classes are not reported.
        assert_eq!(inlet_count("some_external", &[]), None);
        assert_eq!(inlet_count("clip", &[]), None);
        assert_eq!(inlet_count("dac~", &[]), None);
    }

    #[test]
    fn print_has_no_outlets() {
        assert_eq!(outlet_count("print", &[]), Some(0));
    }
}
