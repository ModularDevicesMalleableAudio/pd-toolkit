# Test Fixture Manifest

## Handcrafted Fixtures

Each fixture is manually written to test a specific parser/editor behavior.
Object indices are annotated in comments where relevant.

| File | Tests | Objects | Connections | Key features |
|------|-------|---------|-------------|--------------|
| `minimal.pd` | Smallest valid patch | 0 | 0 | Just canvas header |
| `simple_chain.pd` | Linear A→B→C | 3 | 2 | Basic parse + connect |
| `branching.pd` | Fan-out from trigger | 5 | 4 | Multiple outlets |
| `merging.pd` | Fan-in to one object | 5 | 4 | Multiple sources |
| `cycle.pd` | Feedback loop | 5 | 5 | Cycle detection |
| `nested_subpatch.pd` | 2-level nesting | 3+3 | 2+2 | Depth tracking, restore indexing |
| `deep_subpatch.pd` | 4-level nesting | 3+3+3+3 | 2+2+2+2 | Deep depth stack |
| `multiple_subpatches.pd` | 2 sibling subpatches | 5+3+3 | 4+2+2 | Parallel subpatches at same depth |
| `multiline_obj.pd` | Multi-line msg entry | 3 | 2 | `;` on different line than `#X` |
| `escaped_chars.pd` | `\$`, `\,` escapes | 4 | 3 | Escape preservation |
| `escaped_semicolons.pd` | `\;` in messages | 4 | 3 | `\;` must not split entry |
| `dollar_signs.pd` | `\$1`, `\$2` args | 4 | 2 | Abstraction arguments |
| `with_declare.pd` | Standalone `#X declare` | 10 | 6 | Declare NOT an object |
| `with_width_hint.pd` | `#X f 38;` after restore | 2 | 1 | Width hint NOT an object |
| `with_graph.pd` | Graph-on-parent | 4 | 2 | Graph subpatch + arrays |
| `graph_and_pd_subpatches.pd` | Both subpatch types | 4 | 2 | Mixed graph + pd restore |
| `float_vs_width.pd` | `f` class vs `, f N` | 5 | 5 | Disambiguation |
| `all_gui_types.pd` | tgl/bng/nbx/etc | 12 | 9 | GUI send/receive fields |
| `send_receive.pd` | s/r, s~/r~, throw~/catch~ | 9 | 5 | Send/receive pairs |
| `send_receive_lint.pd` | Lint --send-receive | 7 | 0 | Orphan send, dead receive, matched pair, GUI send, s~/r~ |
| `fan_out_lint.pd` | Lint --fan-out | 6 | 4 | Control fan-out from bng (warn); signal fan-out from osc~ (no warn) |
| `dsp_loop_lint.pd` | Lint --dsp-loop | 7 | 6 | Signal cycle, linear chain, control cycle (no warn) |
| `arrays.pd` | Array definitions | 5 | 2 | Array inventory |
| `orphans.pd` | Unconnected objects | 5 | 1 | Orphan detection |
| `displays.pd` | Connected debug displays | 7 | 4 | Display finder |
| `signal_chain.pd` | Audio-rate objects | 5 | 5 | Signal type inference |
| `with_c_entry.pd` | `#C restore;` entry | 7 | 3 | Non-standard entry handling |
| `large_patch.pd` | 120 objects | 120 | ~130 | Performance baseline |
| `empty_file.pd` | Empty/zero-byte file | 0 | 0 | Graceful error |
| `malformed_missing_semicolon.pd` | Missing `;` | - | - | Parse error reporting |
| `malformed_bad_connection.pd` | Out-of-range connect | 2 | 1 (bad) | Validation error |

## Corpus Fixtures (copied from sequencer repo)

Real-world patches from [malleable808/sequencer](https://gitlab.com/malleable808/sequencer/) that exercise combinations of features. Each entry links to the commit from which the file was copied.

| File | Source | Commit | Key features |
|------|--------|--------|--------------|
| `minimal_real.pd` | `input/seq_trig_in.pd` | [`8d2421d`](https://gitlab.com/malleable808/sequencer/-/commit/8d2421dee186553fa347e47cb870e5677d4dc46b) | Smallest real file (1 line) |
| `send_receive_real.pd` | `seq_abs/midi_muter_alt.pd` | [`172fefa`](https://gitlab.com/malleable808/sequencer/-/commit/172fefae626b64a72d9a2c81b6e8d2f77f4998bf) | s/r with dollar signs |
| `graph_array_real.pd` | `backend/swing_pattern_array.pd` | [`ca898e8`](https://gitlab.com/malleable808/sequencer/-/commit/ca898e8ce131ad2ba6d459878eed0453d3f63cd9) | Graph-on-parent with array data |
| `simple_chain_real.pd` | `seq_abs/sustain_abs/loopround.pd` | [`2762914`](https://gitlab.com/malleable808/sequencer/-/commit/2762914e0bfa9fe7ff25ae124722c50348d5e172) | Simple real chain |
| `escaped_dollar_real.pd` | `seq_abs/allmuter.pd` | [`d30b83b`](https://gitlab.com/malleable808/sequencer/-/commit/d30b83baafb6a3978349277a7070f8932462d0e3) | Dollar sign escaping |
| `declare_real.pd` | `pos_abs/COLOUR_PTN.pd` | [`329f17c`](https://gitlab.com/malleable808/sequencer/-/commit/329f17c7d6731222bb8dbfaf1b23cccf3186849b) | Standalone declare + complex routing |
| `complex_nested_real.pd` | `view_abs/cp_modulator.pd` | [`a449154`](https://gitlab.com/malleable808/sequencer/-/commit/a4491548e530a6bf0b4c72fcf551a6bac0c88dca) | Deep nesting + #X f + graphs + floatatom |
| `c_entry_real.pd` | `seq_abs/prob_abs/ad_abs/drop_length.pd` | [`66f49c5`](https://gitlab.com/malleable808/sequencer/-/commit/66f49c5780f6c17b6e9646285cca0a1539785343) | `#C restore;` entries |
| `escaped_semicolons_real.pd` | `view_abs/io-arrays_exist.pd` | [`b36ba14`](https://gitlab.com/malleable808/sequencer/-/commit/b36ba144f8e91c02956ab8008a9979d6a9b5274e) | `\;` in messages |
| `graph_subpatch_real.pd` | `view_abs/swing_per_sequencer.pd` | [`093dc36`](https://gitlab.com/malleable808/sequencer/-/commit/093dc36715bc0a9f91cd00c124062b669eaa4194) | Many graph subpatches + floatatom |
| `multiline_msg_real.pd` | `view_abs/EMPTY.pd` | [`748c03e`](https://gitlab.com/malleable808/sequencer/-/commit/748c03e00278b5d95e175fe05b0f7ae4ee8be6e0) | Long multi-line messages |
| `deep_nesting_real.pd` | `view_abs/CHORDEDIT.pd` | [`57be5f2`](https://gitlab.com/malleable808/sequencer/-/commit/57be5f27ec15bdc33956b84c2f8a339c095ee4c3) | Deep nesting with arrays |
| `width_hint_real.pd` | `seq_abs/euclidean_abs/euclid_mute.pd` | [`c3564e4`](https://gitlab.com/malleable808/sequencer/-/commit/c3564e4257948b0f9b88ed693eff70a5a2b4664f) | `#X f` after restore |
| `floatatom_real.pd` | `view_abs/CLOCK_DIV_VIEW.pd` | [`843267e`](https://gitlab.com/malleable808/sequencer/-/commit/843267eab49194c0971ed57b91d755e436019760) | floatatom + mixed features |
| `symbolatom_real.pd` | `view_abs/CC.pd` | [`748c03e`](https://gitlab.com/malleable808/sequencer/-/commit/748c03e00278b5d95e175fe05b0f7ae4ee8be6e0) | symbolatom usage |

## Abstraction Fixtures

For dependency analysis testing.

| File | Purpose |
|------|---------|
| `used_abs.pd` | Abstraction that is referenced by `uses_abstractions.pd` |
| `unused_abs.pd` | Abstraction that exists but is not referenced |
| `uses_abstractions.pd` | Patch referencing `used_abs` and `missing_abs` (nonexistent) |

## Object Index Reference

### `with_declare.pd` (critical — tests standalone declare skip)
```
Entry 0: #N canvas (canvas header, no index)
Entry 1: #X declare -path pos_abs (NOT an object, no index)
Entry 2: #X obj inlet           → depth 0, index 0
Entry 3: #X obj declare         → depth 0, index 1
Entry 4: #X text                → depth 0, index 2
Entry 5: #X obj r               → depth 0, index 3
Entry 6: #X msg 0               → depth 0, index 4
Entry 7: #X obj f               → depth 0, index 5
Entry 8: #X obj t b             → depth 0, index 6
Entry 9: #X obj t b             → depth 0, index 7
Entry 10: #X obj outlet         → depth 0, index 8
Entry 11: #X obj t f b          → depth 0, index 9
```
Connection `0 0 9 0` = inlet (index 0) → t f b (index 9) ✓

### `with_width_hint.pd` (critical — tests #X f skip)
```
Depth 0:
  Entry: #N canvas (no index)
  Entry: #N canvas sub (opens depth 1)
  ...subpatch contents at depth 1...
  Entry: #X restore pd my_sub  → depth 0, index 0
  Entry: #X f 38               → NOT an object, no index
  Entry: #X obj print result   → depth 0, index 1
```
Connection `0 0 1 0` = restore (index 0) → print (index 1) ✓

### `nested_subpatch.pd`
```
Depth 0:
  index 0: #X obj inlet
  index 1: #X restore pd my_sub
  index 2: #X obj outlet
Depth 1:
  index 0: #X obj inlet
  index 1: #X obj + 1
  index 2: #X obj outlet
```
