use serde_json::json;

use schemalint::ir::NodeKind;
use schemalint::normalize::{normalize, NormalizeError};

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

include!("normalizer_tests/part_01.rs");
include!("normalizer_tests/part_02.rs");
include!("normalizer_tests/part_03.rs");
include!("normalizer_tests/part_04.rs");
