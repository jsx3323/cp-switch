use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// cp-switch 的单一状态文件结构
#[derive(Serialize, Deserialize, Default)]
pub(crate) struct State {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub profiles: HashMap<String, Value>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub project_currents: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_current: Option<String>,
}
