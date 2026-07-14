use serde::{Deserialize, Serialize};

/// Container for serialized postcard data with an attached `version` field.
#[derive(Clone, Debug, Serialize)]
pub struct UntypedValue<'a> {
    pub version: &'a str,
    pub value: Vec<u8>,
}

/// Container for serialized postcard data with an attached `version` field.
#[derive(Clone, Debug, Deserialize)]
pub struct UntypedValueRef<'a> {
    pub version: &'a str,
    pub value: &'a [u8],
}
